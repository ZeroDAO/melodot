use crate::erasure_coding::*;
use crate::extend_col::extend_segments_col;
use crate::recovery::*;
use crate::segment::*;
use alloc::vec;
use kzg::FFTFr;
use kzg::Fr;
use kzg::G1;
use melo_core_primitives::kzg::BlsScalar;
use melo_core_primitives::kzg::KZGProof;
use melo_core_primitives::kzg::Polynomial;
use melo_core_primitives::kzg::Position;
use melo_core_primitives::kzg::ReprConvert;
use melo_core_primitives::kzg::{embedded_kzg_settings, KZGCommitment, KZG};
use melo_core_primitives::segment::Segment;
use melo_core_primitives::segment::SegmentData;
use rand::seq::SliceRandom;
use rust_kzg_blst::types::fr::FsFr;
use rust_kzg_blst::types::g1::FsG1;
use rust_kzg_blst::types::poly::FsPoly;
use rust_kzg_blst::utils::reverse_bit_order;

fn reverse_bits_limited(length: usize, value: usize) -> usize {
	let unused_bits = length.leading_zeros();
	value.reverse_bits() >> unused_bits
}

fn random_poly(s: usize) -> Polynomial {
	let coeffs = (0..s)
		.map(|_| rand::random::<[u8; 31]>())
		.map(BlsScalar::from)
		.collect::<Vec<_>>();
	let poly = FsPoly { coeffs: BlsScalar::vec_to_repr(coeffs) };
	Polynomial::from(poly)
}

fn random_vec(s: usize) -> Vec<usize> {
	let mut positions: Vec<usize> = (0..s).collect();
	positions.shuffle(&mut rand::thread_rng());
	positions
}

#[test]
fn segment_datas_to_row_test() {
	// 构建随机 segment datas
	let chunk_len: usize = 16;
	let chunk_count: usize = 4;
	let num_shards = chunk_len * chunk_count;
	let mut segment_datas: Vec<Option<SegmentData>> = Vec::new();
	for _ in 0..chunk_count {
		let data = (0..chunk_len)
			.map(|_| rand::random::<[u8; 31]>())
			.map(BlsScalar::from)
			.collect::<Vec<_>>();
		let proof = KZGProof(FsG1::rand());
		let segment_data = SegmentData { data, proof };
		segment_datas.push(Some(segment_data));
	}
	segment_datas[2] = None;
	segment_datas[3] = None;
	// 转换为 row
	let row = segment_datas_to_row(&segment_datas, chunk_len);
	// 验证是否正确
	for i in 0..num_shards {
		let data = match segment_datas[i / chunk_len] {
			Some(ref segment_data) => Some(segment_data.data[i % chunk_len]),
			None => None,
		};
		assert_eq!(row[i], data);
	}
}

#[test]
fn order_segments_col_test() {
	// 构建随机 segment datas
	let chunk_len: usize = 16;
	let k: usize = 4;
	let mut segment_datas: Vec<Option<SegmentData>> = Vec::new();
	for _ in 0..k * 2 {
		let data = (0..chunk_len)
			.map(|_| rand::random::<[u8; 31]>())
			.map(BlsScalar::from)
			.collect::<Vec<_>>();
		let proof = KZGProof(FsG1::rand());
		let segment_data = SegmentData { data, proof };
		segment_datas.push(Some(segment_data));
	}
	segment_datas[2] = None;
	segment_datas[3] = None;
	// 构建segemnts
	let segments_option = segment_datas
		.iter()
		.enumerate()
		.map(|(i, segment_data)| {
			let position = Position { x: 0, y: i as u32 };
			match segment_data {
				Some(segment_data) => Some(Segment { position, content: segment_data.clone() }),
				None => None,
			}
		})
		.collect::<Vec<_>>();
	let segments = segments_option.iter().filter_map(|segment| segment.clone()).collect::<Vec<_>>();
	let mut s_segments = segments.clone();
	// 打乱顺序
	s_segments.shuffle(&mut rand::thread_rng());
	// 转换为 row
	let col: Vec<Option<SegmentData>> = order_segments_col(&s_segments, k).unwrap();
	// 验证是否正确
	for i in 0..k * 2 {
		// segments_option 如果是 some 则比较大小，如果是 None 则直接验证
		if let Some(segment) = &segments_option[i] {
			assert_eq!(col[i], Some(segment.content.clone()));
		} else {
			assert_eq!(col[i], None);
		}
	}
	// 修改 s_segments 其中 1个 x
	s_segments[1].position.x = 3;
	// 转换为 row
	let col: Result<Vec<Option<SegmentData>>, String> = order_segments_col(&s_segments, k);
	// 验证是否失败
	assert!(col.is_err());
	// 向 s_segments 中 push 3个 随机 segment
	for _ in 0..3 {
		let data = (0..chunk_len)
			.map(|_| rand::random::<[u8; 31]>())
			.map(BlsScalar::from)
			.collect::<Vec<_>>();
		let proof = KZGProof(FsG1::rand());
		let segment_data = SegmentData { data, proof };
		let position = Position { x: 0, y: 0 };
		s_segments.push(Segment { position, content: segment_data });
	}
	// 转换为 row
	let col: Result<Vec<Option<SegmentData>>, String> = order_segments_col(&s_segments, k);
	// 验证是否失败
	assert!(col.is_err());

}

#[test]
fn poly_to_segment_vec_test() {
	// Build a random polynomial
	let chunk_len: usize = 16;
	let chunk_count: usize = 4;
	let num_shards = chunk_len * chunk_count;
	let poly = random_poly(num_shards);
	// Get the commitment of poly
	let kzg = KZG::new(embedded_kzg_settings());
	let commitment = kzg.commit(&poly).unwrap();
	// Convert to segments
	let segments = poly_to_segment_vec(&poly, &kzg, 0, chunk_len).unwrap();

	// Verify if it's correct
	for i in 0..chunk_count {
		let verify = segments[i].verify(&kzg, &commitment, chunk_count).unwrap();
		assert!(verify);
	}

	// Convert segments to row
	let mut row = segments
		.into_iter()
		.flat_map(|segment| segment.content.data)
		.collect::<Vec<_>>();
	// Reverse row
	reverse_bit_order(&mut row);
	// Convert row to coefficient form
	let recovery_poly = kzg.get_fs().fft_fr(&BlsScalar::vec_to_repr(row), true).unwrap();
	// Verify if it's correct
	for i in 0..num_shards {
		assert_eq!(recovery_poly[i], poly.0.coeffs[i]);
	}
}

#[test]
fn order_segments_row_test() {
	// Build a random polynomial
	let chunk_len: usize = 16;
	let chunk_count: usize = 4;
	let num_shards = chunk_len * chunk_count;
	let poly = random_poly(num_shards);
	// Get the commitment of poly
	let kzg = KZG::new(embedded_kzg_settings());
	// Convert to segments
	let segments = poly_to_segment_vec(&poly, &kzg, 0, chunk_len).unwrap();
	let mut random_segments: Vec<Option<Segment>> = Vec::new();

	let random_positions = random_vec(3 * chunk_count);
	for i in 0..chunk_count * 2 {
		let position = random_positions[i];
		if position < 2 * chunk_count {
			random_segments.push(Some(segments[position].clone()));
		} else {
			random_segments.push(None);
		}
	}
	// 从 random_segments 获取有效的 segment
	let mut s_segments = random_segments.iter().filter_map(|segment| segment.clone()).collect::<Vec<_>>();
	// 随机洗牌 s_segments
	s_segments.shuffle(&mut rand::thread_rng());
	// Order segments
	let ordered_segments = order_segments_row(&s_segments, chunk_count).unwrap();
	for i in 0..chunk_count * 2 {
		if let Some(segment_data) = &ordered_segments[i] {
			assert_eq!(segment_data.data, segments[i].content.data);
		}
	}
	// 获取 ordered_segments 中 some 的数量，是否和 s_segments 长度相等
	let some_count = ordered_segments.iter().filter(|segment_data| segment_data.is_some()).count();
	assert_eq!(some_count, s_segments.len());
	// 修改 s_segments 其中 1个 y
	s_segments[0].position.y = 3;
	// Order segments
	let ordered_segments = order_segments_row(&s_segments, chunk_count);
	// Verify if it fails
	assert!(ordered_segments.is_err());

	s_segments[0].position.y = 0;
	// 向 s_segments 中 push 2 * 个 随机 segment
	for _ in 0..2 * chunk_count {
		let data = (0..chunk_len)
			.map(|_| rand::random::<[u8; 31]>())
			.map(BlsScalar::from)
			.collect::<Vec<_>>();
		let proof = KZGProof(FsG1::rand());
		let segment_data = SegmentData { data, proof };
		let position = Position { x: 0, y: 0 };
		s_segments.push(Segment { position, content: segment_data });
	}
	// Order segments
	let ordered_segments = order_segments_row(&s_segments, chunk_count);
	// Verify if it fails
	assert!(ordered_segments.is_err());

}

#[test]
fn extend_poly_random_round_trip_test() {
	let kzg = KZG::new(embedded_kzg_settings());
	let num_shards = 16;
	let poly = random_poly(num_shards);

	let extended_poly = extend_poly(kzg.get_fs(), &poly).unwrap();
	assert_eq!(extended_poly.len(), 32);

	let random_positions = random_vec(num_shards * 2);
	let mut cells = [None; 32];
	for i in 0..num_shards {
		let position = random_positions[i];
		cells[position] = Some(extended_poly[position]);
	}
	reverse_bit_order(&mut cells);
	let mut recovered_poly = recover(kzg.get_fs(), &cells.as_slice()).unwrap();
	reverse_bit_order(&mut recovered_poly);
	for i in 0..num_shards * 2 {
		assert_eq!(recovered_poly[i].0, extended_poly[i].0);
	}
}

#[test]
fn recover_segments_random() {
	// Build a random polynomial
	let chunk_len: usize = 16;
	let chunk_count: usize = 4;
	let num_shards = chunk_len * chunk_count;

	let poly = random_poly(num_shards);
	// Convert the polynomial to segments
	let kzg = KZG::new(embedded_kzg_settings());
	let segments: Vec<Segment> = poly_to_segment_vec(&poly, &kzg, 0, chunk_len).unwrap();
	assert_eq!(segments.len(), 8);

	// Take most of them randomly
	let mut random_segments: Vec<Segment> = Vec::new();
	// Get a random Vec of length num_shards, where each number is unique and less than 2 * num_shards
	let random_positions = random_vec(2 * chunk_count);

	for i in 0..chunk_count {
		random_segments.push(segments[random_positions[i]].clone());
	}

	// Recover segments
	let recovered_segments =
		recovery_row_from_segments(&random_segments, &kzg, chunk_count).unwrap();
	assert_eq!(recovered_segments[0], segments[0]);

	// Verify if the recovered segments are the same as the original segments
	for i in 0..chunk_count {
		assert_eq!(recovered_segments[i], segments[i]);
	}

	// Remove one segment from random_segments
	random_segments.remove(0);
	// Recover segments
	let recovered_segments = recovery_row_from_segments(&random_segments, &kzg, chunk_count);

	// Verify if it fails
	assert!(recovered_segments.is_err());
}

#[test]
fn commit_multi_random() {
	let chunk_len: usize = 16;
	let chunk_count: usize = 4;
	let num_shards = chunk_len * chunk_count;

	let kzg = KZG::new(embedded_kzg_settings());

	let poly = random_poly(num_shards);

	// Commit to the polynomial
	let commitment = kzg.commit(&poly).unwrap();
	// Compute the multi proofs
	let proofs = kzg.all_proofs(&poly, chunk_len).unwrap();

	let mut extended_coeffs = poly.0.coeffs.clone();

	extended_coeffs.resize(poly.0.coeffs.len() * 2, FsFr::zero());

	let mut extended_coeffs_fft = kzg.get_fs().fft_fr(&extended_coeffs, false).unwrap();

	reverse_bit_order(&mut extended_coeffs_fft);

	// Verify the proofs
	let mut ys = vec![FsFr::default(); chunk_len];
	for pos in 0..(2 * chunk_count) {
		// The ys from the extended coeffients
		for i in 0..chunk_len {
			ys[i] = extended_coeffs_fft[chunk_len * pos + i].clone();
		}
		reverse_bit_order(&mut ys);

		// Verify this proof
		let result = kzg
			.check_proof_multi(&commitment, pos, chunk_count, &ys, &proofs[pos], chunk_len)
			.unwrap();
		assert!(result);
	}
}

#[test]
fn extend_and_commit_multi_random() {
	let chunk_len: usize = 16;
	let chunk_count: usize = 4;
	let num_shards = chunk_len * chunk_count;

	let kzg = KZG::new(embedded_kzg_settings());

	let evens = (0..num_shards)
		.map(|_| rand::random::<[u8; 31]>())
		.map(BlsScalar::from)
		.collect::<Vec<_>>();

	let odds = extend(&kzg.get_fs(), &evens).unwrap();

	let mut data = Vec::new();
	for i in (0..num_shards * 2).step_by(2) {
		data.push(evens[i / 2].clone());
		data.push(odds[i / 2].clone());
	}

	let coeffs = kzg.get_fs().fft_fr(BlsScalar::slice_to_repr(&data), true).unwrap();

	for coeff in coeffs.iter().take(num_shards * 2).skip(num_shards) {
		assert!(coeff.is_zero());
	}

	let mut poly: Polynomial = Polynomial::new(num_shards).unwrap();
	for i in 0..num_shards {
		poly.0.coeffs[i] = FsFr::from(coeffs[i]);
	}

	// Commit to the polynomial
	let commitment = kzg.commit(&poly).unwrap();
	// Compute the multi proofs
	let proofs = kzg.all_proofs(&poly, chunk_len).unwrap();

	reverse_bit_order(&mut data);
	let mut ys = vec![FsFr::default(); chunk_len];
	for pos in 0..(2 * chunk_count) {
		// The ys from the extended coeffients
		for i in 0..chunk_len {
			ys[i] = BlsScalar::vec_to_repr(data.clone())[chunk_len * pos + i].clone();
		}
		reverse_bit_order(&mut ys);

		// Verify this proof
		let result = kzg
			.check_proof_multi(&commitment, pos, chunk_count, &ys, &proofs[pos], chunk_len)
			.unwrap();
		assert!(result);
	}
}

#[test]
fn extend_fs_g1_random() {
	let kzg = KZG::new(embedded_kzg_settings());
	let mut commits: Vec<KZGCommitment> = Vec::new();
	for _rep in 0..4 {
		commits.push(KZGCommitment(FsG1::rand()));
	}
	let extended_commits = extend_fs_g1(kzg.get_fs(), &commits).unwrap();
	assert!(extended_commits.len() == 8);
	assert!(extended_commits[2].0 == commits[1].0);
}

#[test]
fn extend_segments_col_random() {
	// Build multiple polynomials with random coefficients
	let chunk_len: usize = 16;
	let chunk_count: usize = 4;
	let num_shards = chunk_len * chunk_count;
	let k: usize = 4;
	let polys = (0..k).map(|_| random_poly(num_shards)).collect::<Vec<_>>();
	// Commit to all polynomials
	let kzg = KZG::new(embedded_kzg_settings());
	let commitments = polys.iter().map(|poly| kzg.commit(poly).unwrap()).collect::<Vec<_>>();
	// Extend polynomial commitments to twice the size
	let extended_commitments = extend_fs_g1(kzg.get_fs(), &commitments).unwrap();
	// Convert all polynomials to segments
	let matrix = polys
		.iter()
		.enumerate()
		.map(|(i, poly)| poly_to_segment_vec(&poly, &kzg, i, chunk_len).unwrap())
		.collect::<Vec<_>>();
	assert!(matrix[0][0].verify(&kzg, &commitments[0], chunk_count).unwrap());
	// Pick a column from the segments
	let pick_col_index: usize = 1;
	let col = matrix.iter().map(|row| row[pick_col_index].clone()).collect::<Vec<_>>();
	// Extend the column
	let extended_col = extend_segments_col(kzg.get_fs(), &col).unwrap();

	for i in 0..(chunk_count) {
		let pick_s = extended_col[i].clone();
		assert!(pick_s.verify(&kzg, &extended_commitments[i * 2 + 1], chunk_count).unwrap());
	}
}
