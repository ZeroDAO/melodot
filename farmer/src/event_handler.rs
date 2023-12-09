// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use std::time::Instant;

use anyhow::{anyhow, Context};
use futures::lock::Mutex;
use log::{error, info};
use melo_das_network::Arc;
use melo_das_primitives::{Position, SafeScalar, Segment};
use meloxt::{Client, MelodotHeader as Header};
use subxt::{
	config::{substrate::BlakeTwo256, Hasher},
	utils::H256,
};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

use melo_core_primitives::{
	config::{EXTENDED_SEGMENTS_PER_BLOB, FIELD_ELEMENTS_PER_SEGMENT, PRE_CELL_LEADING_ZEROS},
	traits::HeaderWithCommitment,
};
use melo_das_db::sqlite::SqliteDasDb;
use melo_daser::{DasKv, DasNetworkServiceWrapper, FetchData, SamplingClient};
use melo_proof_of_space::{find_solutions, Piece, PiecePosition, PreCell, Solution};

pub async fn run<H: HeaderWithCommitment + Sync>(
	rpc_client: Client,
	message_tx: Sender<(Header, Instant)>,
	error_sender: Sender<anyhow::Error>,
	network: DasNetworkServiceWrapper,
	database: Arc<Mutex<SqliteDasDb>>,
) {
	let client: SamplingClient<H, SqliteDasDb, DasNetworkServiceWrapper> =
		SamplingClient::new(network, database.clone());

	let signer = rpc_client.signer.public_key();
	let farmer_id = BlakeTwo256::hash(signer.as_ref());

	let mut new_heads_sub = match rpc_client.api.blocks().subscribe_best().await {
		Ok(subscription) => {
			info!("🌐 Subscribed to best block headers");
			subscription
		},
		Err(e) => {
			error!("⚠️ Failed to subscribe to best blocks: {:?}", e);
			return
		},
	};

	while let Some(message) = new_heads_sub.next().await {
		let received_at = Instant::now();
		if let Ok(block) = message {
			let header = block.header().clone();

			let block_number = header.number;
			let block_hash = header.hash();

			let rows_count = header.col_num().unwrap_or_default();

			info!("✅ Received best block header #{}", block_number.clone());

			let message = (header.clone(), received_at);
			if let Err(error) = message_tx.send(message).await.context("Send failed") {
				error!("❌ Fail to process best block header: {error}");
			}

			let row_inds = Solution::<H256, u32>::select_indices(
				&farmer_id,
				&block_hash,
				(rows_count * 2) as usize,
				1,
			);

			let col_inds = Solution::<H256, u32>::select_indices(
				&farmer_id,
				&block_hash,
				EXTENDED_SEGMENTS_PER_BLOB,
				1,
			);

			let fetch_result = tokio::try_join!(
				client.fetch_rows(&header, &row_inds),
				client.fetch_cols(&header, &col_inds)
			);

			let ((rows, _), (cols, _, _)) = match fetch_result {
				Ok((rows, cols)) => (rows, cols),
				Err(e) => {
					error!("❌ Error fetching rows and cols: {:?}", e);
					continue
				},
			};

			let mut pre_cells: Vec<PreCell> = Vec::new();

			rows.iter().for_each(|row| {
				process_segment_data(row, &farmer_id, PiecePosition::from_row, &mut pre_cells);
			});
			cols.iter().for_each(|col| {
				process_segment_data(col, &farmer_id, PiecePosition::from_column, &mut pre_cells);
			});

			let mut solutions: Vec<Solution<H256, u32>> = Vec::new();

			let mut database_guard = database.lock().await;

			pre_cells.iter().for_each(|pre_cell| {
				match find_solutions(&mut *database_guard, &farmer_id, pre_cell, &block_hash) {
					Ok(mut ss) => {
						solutions.append(&mut ss);
					},
					Err(e) => {
						error!("❌ Error finding solutions: {:?}", e);
					},
				};
			});

			process_segments(
				&rows,
				block_number,
				&mut *database_guard,
				&farmer_id,
				PiecePosition::from_row,
			);

			process_segments(
				&cols,
				block_number,
				&mut *database_guard,
				&farmer_id,
				PiecePosition::from_column,
			);
		} else if let Err(e) = message {
			error!("❌ Error receiving best header message: {:?}", e);
		}
	}

	if let Err(error) = error_sender.send(anyhow!("Best blocks subscription disconnected")).await {
		error!("🚫 Cannot send error to error channel: {error}");
	}
}

fn process_segments<F>(
	segments: &[Option<Segment>],
	block_number: u32,
	db: &mut impl DasKv,
	farmer_id: &H256,
	piece_position_fn: F,
) where
	F: Fn(&Position) -> PiecePosition,
{
	segments.chunks(EXTENDED_SEGMENTS_PER_BLOB).for_each(|chunk| {
		let segment_vec = chunk.iter().filter_map(|seg| seg.clone()).collect::<Vec<_>>();
		if let Some(first_segment) = segment_vec.first() {
			let piece_position = piece_position_fn(&first_segment.position);
			let piece = Piece::new(block_number, piece_position, &segment_vec);

			if let Err(e) = piece.save(db, farmer_id) {
				error!("❌ Error to save piece : {:?}", e);
			}
		}
	});
}

fn process_segment_data<F>(
	segment: &Option<Segment>,
	farmer_id: &H256,
	position_fn: F,
	pre_cells: &mut Vec<PreCell>,
) where
	F: Fn(&Position) -> PiecePosition,
{
	if let Some(seg) = segment {
		seg.content.data.iter().enumerate().for_each(|(i, data)| {
			if Solution::<H256, u32>::check_pre_cell(
				&data.to_bytes_safe(),
				farmer_id,
				PRE_CELL_LEADING_ZEROS,
			) {
				let piece_position = position_fn(&seg.position);
				let offset = calculate_offset(&seg.position, i, &piece_position);
				pre_cells.push(PreCell::new(
					piece_position,
					seg.content.proof,
					data,
					offset,
				));
			}
		});
	}
}

fn calculate_offset(
	segment_position: &Position,
	index: usize,
	piece_position: &PiecePosition,
) -> u32 {
	match piece_position {
		PiecePosition::Row(_) =>
			segment_position.x * FIELD_ELEMENTS_PER_SEGMENT as u32 + index as u32,
		PiecePosition::Column(_) =>
			segment_position.y * FIELD_ELEMENTS_PER_SEGMENT as u32 + index as u32,
	}
}
