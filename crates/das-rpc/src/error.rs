// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! DAS RPC errors.

// use jsonrpsee::{
//     core::Error as JsonRpseeError,
//     types::error::{CallError, ErrorObject},
// };

use jsonrpsee::types::{error::ErrorObject, ErrorObjectOwned};

/// DAS RPC errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Decoding extrinsic failed
	#[error("Decoding extrinsic failed: {}", .0)]
	DecodingExtrinsicFailed(Box<dyn std::error::Error + Send + Sync>),
	/// Decoding transaction metadata failed
	#[error("Decoding transaction metadata failed: {}", .0)]
	DecodingTransactionMetadataFailed(Box<dyn std::error::Error + Send + Sync>),
	/// Failed to fetch transaction metadata details
	#[error("Failed to fetch transaction metadata details: {}", .0)]
	FetchTransactionMetadataFailed(Box<dyn std::error::Error + Send + Sync>),
	/// Invalid transaction format
	#[error("Invalid transaction format")]
	InvalidTransactionFormat,
	/// Data length or hash error
	#[error("Data length error")]
	DataLength,
	/// DAS network error
	#[error("DAS network error")]
	FailedToRemoveRecords,
	/// Failed to push transaction
	#[error("Failed to push transaction: {}", .0)]
	TransactionPushFailed(Box<dyn std::error::Error + Send + Sync>),
}

/// DAS error codes
const BASE_ERROR: i32 = 10000;

impl From<Error> for ErrorObjectOwned {
	fn from(e: Error) -> Self {
		match e {
			Error::DecodingExtrinsicFailed(e) => ErrorObject::owned(
				BASE_ERROR + 1,
				"Decoding extrinsic failed",
				Some(format!("{:?}", e)),
			),
			Error::DecodingTransactionMetadataFailed(e) => ErrorObject::owned(
				BASE_ERROR + 2,
				"Decoding transaction metadata failed",
				Some(format!("{:?}", e)),
			),
			Error::FetchTransactionMetadataFailed(e) => ErrorObject::owned(
				BASE_ERROR + 3,
				"Failed to fetch transaction metadata details",
				Some(format!("{:?}", e)),
			),
			Error::InvalidTransactionFormat => {
				ErrorObject::owned(BASE_ERROR + 4, "Invalid transaction format", None::<()>)
			},
			Error::DataLength => ErrorObject::owned(
				BASE_ERROR + 5,
				"Data/Commitments/Proofs length error",
				None::<()>,
			),
			Error::TransactionPushFailed(e) => ErrorObject::owned(
				BASE_ERROR + 6,
				"Failed to push transaction",
				Some(format!("{:?}", e)),
			),
			Error::FailedToRemoveRecords => {
				ErrorObject::owned(BASE_ERROR + 7, "Failed to remove records", None::<()>)
			},
		}
	}
}
