use crate::erasure_coding::*;
use crate::recovery::*;
use crate::segment::*;
// use blst_rust::types::g1::FsG1;
use alloc::vec;
use core::slice::Chunks;
use kzg::FFTFr;
use kzg::Fr;
use melo_core_primitives::kzg::BlsScalar;
use melo_core_primitives::kzg::Polynomial;
use melo_core_primitives::kzg::ReprConvert;
use melo_core_primitives::kzg::{embedded_kzg_settings, KZG};
use rust_kzg_blst::types::fr::FsFr;
use rust_kzg_blst::utils::reverse_bit_order;
use std::iter;
use std::num::NonZeroUsize;
// use subspace_core_primitives::crypto::kzg::Commitment;
// use subspace_core_primitives::crypto::Scalar;

fn reverse_bits_limited(length: usize, value: usize) -> usize {
	let unused_bits = length.leading_zeros();
	value.reverse_bits() >> unused_bits
}

// TODO: This could have been done in-place, once implemented can be exposed as a utility
fn concatenated_to_interleaved<T>(input: Vec<T>) -> Vec<T>
where
	T: Clone,
{
	if input.len() <= 1 {
		return input;
	}

	let (first_half, second_half) = input.split_at(input.len() / 2);

	first_half.iter().zip(second_half).flat_map(|(a, b)| [a, b]).cloned().collect()
}

// TODO: This could have been done in-place, once implemented can be exposed as a utility
fn interleaved_to_concatenated<T>(input: Vec<T>) -> Vec<T>
where
	T: Clone,
{
	let first_half = input.iter().step_by(2);
	let second_half = input.iter().skip(1).step_by(2);

	first_half.chain(second_half).cloned().collect()
}
// pub fn recovery_row_from_segments(
// 	segments: &Vec<Segment>,
// 	kzg: &KZG,
// ) -> Result<Vec<Segment>, String> {
#[test]
fn recovery_row_from_segments_test() {
	let scale = NonZeroUsize::new(4).unwrap();
	let kzg = KZG::new(embedded_kzg_settings());
	let num_shards = 2usize.pow(scale.get() as u32);
	let source_shards = (0..num_shards / 2)
		.map(|_| rand::random::<[u8; 31]>())
		.map(BlsScalar::from)
		.collect::<Vec<_>>();

	let parity_shards = extend(kzg.get_fs(), &source_shards).unwrap();

	let partial_shards = concatenated_to_interleaved(
		iter::repeat(None)
			.take(num_shards / 4)
			.chain(source_shards.iter().skip(num_shards / 4).copied().map(Some))
			.chain(parity_shards.iter().take(num_shards / 4).copied().map(Some))
			.chain(iter::repeat(None).take(num_shards / 4))
			.collect::<Vec<_>>(),
	);

	let recovered = interleaved_to_concatenated(recover(kzg.get_fs(), &partial_shards).unwrap());

	assert_eq!(recovered, source_shards.iter().chain(&parity_shards).copied().collect::<Vec<_>>());
}

#[test]
fn commit_multi_test() {
	let chunk_len: usize = 16;
	let chunk_count: usize = 4;
	let num_shards = chunk_len * chunk_count;

	let kzg = KZG::new(embedded_kzg_settings());

	let s = (0..num_shards)
		.map(|_| rand::random::<[u8; 31]>())
		.map(BlsScalar::from)
		.collect::<Vec<_>>();
	let mut poly: Polynomial = Polynomial::new(num_shards).unwrap();
	for i in 0..num_shards {
		poly.0.coeffs[i] = FsFr::from(s[i]);
	}

	// Commit to the polynomial
	let commitment = kzg.commit(&poly).unwrap();
	// Compute the multi proofs
	let proofs = kzg.all_proofs(&poly).unwrap();

	let mut extended_coeffs = vec![FsFr::zero(); 2 * num_shards];
	for (i, extended_coeff) in extended_coeffs.iter_mut().enumerate().take(num_shards) {
		*extended_coeff = *s[i];
	}

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
fn extend_and_commit_multi_test() {
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
	let proofs = kzg.all_proofs(&poly).unwrap();

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


// #[test]
// fn basic_data() {
//     let scale = NonZeroUsize::new(8).unwrap();
//     let num_shards = 2usize.pow(scale.get() as u32);
//     let ec = ErasureCoding::new(scale).unwrap();

//     let source_shards = (0..num_shards / 2)
//         .map(|_| rand::random::<[u8; Scalar::SAFE_BYTES]>())
//         .map(Scalar::from)
//         .collect::<Vec<_>>();

//     let parity_shards = ec.extend(&source_shards).unwrap();

//     assert_ne!(source_shards, parity_shards);

//     let partial_shards = concatenated_to_interleaved(
//         iter::repeat(None)
//             .take(num_shards / 4)
//             .chain(source_shards.iter().skip(num_shards / 4).copied().map(Some))
//             .chain(parity_shards.iter().take(num_shards / 4).copied().map(Some))
//             .chain(iter::repeat(None).take(num_shards / 4))
//             .collect::<Vec<_>>(),
//     );

//     let recovered = interleaved_to_concatenated(ec.recover(&partial_shards).unwrap());

//     assert_eq!(
//         recovered,
//         source_shards
//             .iter()
//             .chain(&parity_shards)
//             .copied()
//             .collect::<Vec<_>>()
//     );
// }

// #[test]
// fn basic_commitments() {
//     let scale = NonZeroUsize::new(7).unwrap();
//     let num_shards = 2usize.pow(scale.get() as u32);
//     let ec = ErasureCoding::new(scale).unwrap();

//     let source_commitments = (0..num_shards / 2)
//         .map(|_| Commitment::from(FsG1::rand()))
//         .collect::<Vec<_>>();

//     let parity_commitments = ec.extend_commitments(&source_commitments).unwrap();

//     assert_eq!(source_commitments.len() * 2, parity_commitments.len());

//     // Even indices must be source
//     assert_eq!(
//         source_commitments,
//         parity_commitments
//             .iter()
//             .step_by(2)
//             .copied()
//             .collect::<Vec<_>>()
//     );
// }

// #[test]
// fn bad_shards_number() {
//     let scale = NonZeroUsize::new(8).unwrap();
//     let num_shards = 2usize.pow(scale.get() as u32);
//     let ec = ErasureCoding::new(scale).unwrap();

//     let source_shards = vec![Default::default(); num_shards - 1];

//     assert!(ec.extend(&source_shards).is_err());

//     let partial_shards = vec![Default::default(); num_shards - 1];
//     assert!(ec.recover(&partial_shards).is_err());
// }

// #[test]
// fn not_enough_partial() {
//     let scale = NonZeroUsize::new(8).unwrap();
//     let num_shards = 2usize.pow(scale.get() as u32);
//     let ec = ErasureCoding::new(scale).unwrap();

//     let mut partial_shards = vec![None; num_shards];

//     // Less than half is not sufficient
//     partial_shards
//         .iter_mut()
//         .take(num_shards / 2 - 1)
//         .for_each(|maybe_scalar| {
//             maybe_scalar.replace(Scalar::default());
//         });
//     assert!(ec.recover(&partial_shards).is_err());

//     // Any half is sufficient
//     partial_shards
//         .last_mut()
//         .unwrap()
//         .replace(Scalar::default());
//     assert!(ec.recover(&partial_shards).is_ok());
// }
