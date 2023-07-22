use crate::erasure_coding::*;
use crate::recovery::*;
use crate::segment::*;
// use blst_rust::types::g1::FsG1;
use alloc::vec;
use kzg::FFTFr;
use kzg::Fr;
use melo_core_primitives::kzg::BlsScalar;
use melo_core_primitives::kzg::Polynomial;
use melo_core_primitives::kzg::ReprConvert;
use melo_core_primitives::kzg::{embedded_kzg_settings, KZG};
use rust_kzg_blst::types::fr::FsFr;
use rust_kzg_blst::utils::reverse_bit_order;

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