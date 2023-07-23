use crate::erasure_coding::*;
use crate::extend_col::extend_segments_col;
use crate::recovery::*;
use crate::segment::*;
// use blst_rust::types::g1::FsG1;
use alloc::vec;
use kzg::G1;
use rust_kzg_blst::types::g1::FsG1;
use core::slice::Chunks;
use kzg::FFTFr;
use kzg::Fr;
use melo_core_primitives::kzg::BlsScalar;
use melo_core_primitives::kzg::Polynomial;
use melo_core_primitives::kzg::ReprConvert;
use melo_core_primitives::kzg::{embedded_kzg_settings, KZG, KZGCommitment};
use rust_kzg_blst::types::fr::FsFr;
use rust_kzg_blst::types::poly::FsPoly;
use rust_kzg_blst::utils::reverse_bit_order;
use std::iter;
use std::num::NonZeroUsize;

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

// #[test]
// fn recovery_row_from_segments_test() {
// 	let scale = NonZeroUsize::new(4).unwrap();
// 	let kzg = KZG::new(embedded_kzg_settings());
// 	let num_shards = 2usize.pow(scale.get() as u32);
// 	let source_shards = (0..num_shards / 2)
// 		.map(|_| rand::random::<[u8; 31]>())
// 		.map(BlsScalar::from)
// 		.collect::<Vec<_>>();

// 	let parity_shards = extend(kzg.get_fs(), &source_shards).unwrap();

// 	let partial_shards = concatenated_to_interleaved(
// 		iter::repeat(None)
// 			.take(num_shards / 4)
// 			.chain(source_shards.iter().skip(num_shards / 4).copied().map(Some))
// 			.chain(parity_shards.iter().take(num_shards / 4).copied().map(Some))
// 			.chain(iter::repeat(None).take(num_shards / 4))
// 			.collect::<Vec<_>>(),
// 	);

// 	let recovered = interleaved_to_concatenated(recover(kzg.get_fs(), &partial_shards).unwrap());

// 	assert_eq!(recovered, source_shards.iter().chain(&parity_shards).copied().collect::<Vec<_>>());
// }

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
	let proofs = kzg.all_proofs(&poly,chunk_len).unwrap();

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