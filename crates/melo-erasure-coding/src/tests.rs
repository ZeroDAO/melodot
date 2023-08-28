use crate::bytes_vec_to_blobs;
use crate::erasure_coding::*;
use crate::extend_col::*;
use crate::recovery::*;
use crate::segment::*;

use alloc::vec;
use kzg::FFTFr;
use kzg::Fr;
use kzg::G1;

use melo_das_primitives::blob::Blob;
use melo_das_primitives::crypto::BlsScalar;
use melo_das_primitives::crypto::KZGProof;
use melo_das_primitives::crypto::Position;
use melo_das_primitives::crypto::ReprConvert;
use melo_das_primitives::crypto::{KZGCommitment, KZG};
use melo_das_primitives::polynomial::Polynomial;
use melo_das_primitives::segment::Segment;
use melo_das_primitives::segment::SegmentData;

use rand::seq::SliceRandom;
use rand::Rng;

use rust_kzg_blst::types::fr::FsFr;
use rust_kzg_blst::types::g1::FsG1;
use rust_kzg_blst::types::poly::FsPoly;
use rust_kzg_blst::utils::reverse_bit_order;

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

fn random_bytes(len: usize) -> Vec<u8> {
	let mut rng = rand::thread_rng();
	let bytes: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
	bytes
}

fn blob_proof_case(field_elements_per_blob: usize, minimize: usize) {
    // Build a random blob
    let blob_data_len: usize = 31 * field_elements_per_blob;

    let actual_byte_len = blob_data_len - minimize;
    let blob_data = random_bytes(actual_byte_len);

    let blob = Blob::try_from_bytes_pad(&blob_data, blob_data_len).unwrap();
    let kzg = KZG::default_embedded();
    let commitment = blob.commit(&kzg).unwrap();
    let poly = blob.to_poly();
    let commitment_poly = kzg.commit(&poly).unwrap();

    assert!(commitment_poly == commitment);
    // Calculate the proof for the blob
    let (commitment, proof) =
        blob.commit_and_proof(&kzg, field_elements_per_blob).unwrap();
    // Verify the proof
    let result = blob
        .verify(&kzg, &commitment, &proof, field_elements_per_blob)
        .unwrap();

    assert!(commitment_poly == commitment);
    assert!(result);

    // Modify the value of the proof, verification fails
    let proof_mut = KZGProof(proof.0.add(&FsG1::rand()));
    // Verification fails
    let verify = blob
        .verify(&kzg, &commitment, &proof_mut, field_elements_per_blob)
        .unwrap();
    assert!(!verify);
    // Modify a value in the commit, verification fails
    let commitment_mut = KZGCommitment(commitment.0.add(&FsG1::rand()));
    let verify = blob
        .verify(&kzg, &commitment_mut, &proof, field_elements_per_blob)
        .unwrap();
    assert!(!verify);
    // Modify the blob
    let blob_data = random_bytes(blob_data_len);
    let blob = Blob::try_from_bytes_pad(&blob_data, blob_data_len).unwrap();

    // Verification of the blob's proof fails
    let verify = blob
        .verify(&kzg, &commitment, &proof, field_elements_per_blob)
        .unwrap();
    assert!(!verify);
}

#[test]
fn test_blob_proof() {
    // Test case 1
    blob_proof_case(4096, 0);

    // Test case 2
    blob_proof_case(4, 0);

    // Test case 3: Length less than half
    blob_proof_case(64, (64 / 2 + 1) * 31);

    // Test case 4
    blob_proof_case(64, 50 * 31);

    // Test case 5: Empty blob
    blob_proof_case(4, 4 * 31);
}

#[test]
fn test_recover_poly() {
    // Build a random polynomial
    let num_shards: usize = 16;
    let poly = random_poly(num_shards);

    // Extend the polynomial
    let kzg = KZG::default_embedded();
    let extended_poly = extend_poly(kzg.get_fs(), &poly).unwrap();

    // Remove some elements from it
    let mut shards: Vec<Option<BlsScalar>> =
        extended_poly.iter().map(|shard| Some(*shard)).collect();

    // All shards are Some()
    let mut shards_all_some = shards.clone();
    reverse_bit_order(&mut shards_all_some);
    let recovered_poly_all_some = recover_poly(kzg.get_fs(), &shards_all_some).unwrap();

    let random_positions = random_vec(num_shards * 3);
    for i in 0..2 * num_shards {
        let position = random_positions[i];
        if random_positions[position] > 2 * num_shards {
            shards[i] = None;
        }
    }
    // Reverse the shards
    reverse_bit_order(&mut shards);
    // Recover the polynomial
    let recovered_poly = recover_poly(kzg.get_fs(), &shards).unwrap();
    // Verify if it is correct
    for i in 0..num_shards {
        assert_eq!(recovered_poly.0.coeffs[i], poly.0.coeffs[i]);
        assert_eq!(recovered_poly_all_some.0.coeffs[i], poly.0.coeffs[i]);
    }

    // Set half of the shards to None
    for i in 0..num_shards {
        shards[i] = None;
    }
    // Recover the polynomial
    let recovered_poly = recover_poly(kzg.get_fs(), &shards);
    // Verify if it fails
    assert!(recovered_poly.is_err());
}

#[test]
fn test_blob_verify_batch() {
    // Build a random blob vector
    let blob_count: usize = 4;
    let field_elements_per_blob: usize = 4096;
    let blob_data_len: usize = 31 * field_elements_per_blob;
    let mut blobs: Vec<Blob> = Vec::new();
    for _ in 0..blob_count {
        let blob_data = random_bytes(blob_data_len);
        let blob = Blob::try_from_bytes_pad(&blob_data, blob_data_len).unwrap();
        blobs.push(blob);
    }

    // Commit and get proof for each blob
    let mut commitments: Vec<KZGCommitment> = Vec::new();
    let mut proofs: Vec<KZGProof> = Vec::new();
    let kzg = KZG::default_embedded();
    for blob in blobs.iter() {
        let (commitment, proof) =
            blob.commit_and_proof(&kzg, field_elements_per_blob).unwrap();
        commitments.push(commitment);
        proofs.push(proof);
    }

    // Batch verify commitments and proofs
    let result = Blob::verify_batch(
        &blobs,
        &commitments,
        &proofs,
        &kzg,
        field_elements_per_blob,
    )
    .unwrap();
    assert!(result);
}

fn blob_bytes_conversion_case(field_elements_per_blob: usize, minimize: usize) {
    let blob_data_len: usize = 31 * field_elements_per_blob;

    // Build a random bytes array of length `actual_byte_len`
    let actual_byte_len = blob_data_len - minimize;
    let bytes = random_bytes(actual_byte_len);

    // Convert bytes to a blob
    let blob = Blob::try_from_bytes_pad(&bytes, blob_data_len).unwrap();
    assert_eq!(blob.len(), field_elements_per_blob);

    // Convert the blob back to bytes
    let bytes2 = blob.to_bytes_by_len(actual_byte_len);

    // Verify if bytes are equal
    assert_eq!(bytes, bytes2);

    // Convert the blob back to bytes with a different length
    let bytes3 = blob.to_bytes_by_len(blob_data_len + 100);

    // Check if bytes3 is equal or not depending on the `minimize` value
    if minimize == 0 {
        assert_eq!(bytes3, bytes);
    } else {
        assert_ne!(bytes3, bytes);
    }
}

#[test]
fn test_blob_bytes_conversion() {
	blob_bytes_conversion_case(4096, 0);
	blob_bytes_conversion_case(64, 12);
	blob_bytes_conversion_case(4, 0);
	blob_bytes_conversion_case(32, 2);
	// Empty blob
	blob_bytes_conversion_case(4, 4 * 31);
}

#[test]
fn test_segment_datas_to_row() {
    // Build random segment datas
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

    // Convert to row
    let row = segment_datas_to_row(&segment_datas, chunk_len);

    // Verify if it is correct
    for i in 0..num_shards {
        let data = match segment_datas[i / chunk_len] {
            Some(ref segment_data) => Some(segment_data.data[i % chunk_len]),
            None => None,
        };
        assert_eq!(row[i], data);
    }
}

#[test]
fn test_order_segments_col() {
    // Build random segment datas
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

    // Build segments
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

    // Shuffle the segments to change the order
    s_segments.shuffle(&mut rand::thread_rng());

    // Convert to column order
    let col: Vec<Option<SegmentData>> = order_segments_col(&s_segments, k).unwrap();

    // Verify if it is correct
    for i in 0..k * 2 {
        // segments_option: if it's Some, then compare the content; if it's None, then directly verify
        if let Some(segment) = &segments_option[i] {
            assert_eq!(col[i], Some(segment.content.clone()));
        } else {
            assert_eq!(col[i], None);
        }
    }

    // Modify a single x value in s_segments
    s_segments[1].position.x = 3;

    // Convert to column order, it should fail due to incorrect x values
    let col: Result<Vec<Option<SegmentData>>, String> = order_segments_col(&s_segments, k);
    assert!(col.is_err());

    // Add 3 random segments to s_segments
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

    // Convert to column order, it should fail due to incorrect segment count
    let col: Result<Vec<Option<SegmentData>>, String> = order_segments_col(&s_segments, k);
    assert!(col.is_err());
}

#[test]
fn test_poly_to_segment_vec() {
    // Build a random polynomial
    let chunk_len: usize = 16;
    let chunk_count: usize = 4;
    let num_shards = chunk_len * chunk_count;
    let poly = random_poly(num_shards);

    // Get the commitment of poly
    let kzg = KZG::default_embedded();
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
fn test_order_segments_row() {
    // Build a random polynomial
    let chunk_len: usize = 16;
    let chunk_count: usize = 4;
    let num_shards = chunk_len * chunk_count;
    let poly = random_poly(num_shards);

    // Get the commitment of poly
    let kzg = KZG::default_embedded();

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

    // Get valid segments from random_segments
    let mut s_segments =
        random_segments.iter().filter_map(|segment| segment.clone()).collect::<Vec<_>>();

    // Randomly shuffle s_segments
    s_segments.shuffle(&mut rand::thread_rng());

    // Order segments
    let ordered_segments = order_segments_row(&s_segments, chunk_count).unwrap();

    // Verify if the order is correct
    for i in 0..chunk_count * 2 {
        if let Some(segment_data) = &ordered_segments[i] {
            assert_eq!(segment_data.data, segments[i].content.data);
        }
    }

    // Check if the count of Some segments in ordered_segments is equal to the length of s_segments
    let some_count = ordered_segments.iter().filter(|segment_data| segment_data.is_some()).count();
    assert_eq!(some_count, s_segments.len());

    // Modify one y value in s_segments to make it invalid
    s_segments[0].position.y = 3;

    // Order segments, it should fail due to incorrect y values
    let ordered_segments = order_segments_row(&s_segments, chunk_count);
    assert!(ordered_segments.is_err());

    // Reset the y value to a valid value
    s_segments[0].position.y = 0;

    // Add 2 * chunk_count random segments to s_segments
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

    // Order segments, it should fail due to an incorrect number of segments
    let ordered_segments = order_segments_row(&s_segments, chunk_count);
    assert!(ordered_segments.is_err());
}

#[test]
fn test_extend_poly() {
    let kzg = KZG::default_embedded();
    let num_shards = 16;
    let poly = random_poly(num_shards);

    let extended_poly = extend_poly(kzg.get_fs(), &poly).unwrap();
    assert_eq!(extended_poly.len(), 32);

    let poly_err = random_poly(3);
    let extended_poly_err = extend_poly(kzg.get_fs(), &poly_err);
    assert!(extended_poly_err.is_err());

    let poly_err = random_poly(6);
    let extended_poly_err = extend_poly(kzg.get_fs(), &poly_err);
    assert!(extended_poly_err.is_err());

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
fn test_recovery_row_from_segments() {
    // Build a random polynomial
    let chunk_len: usize = 16;
    let chunk_count: usize = 4;
    let num_shards = chunk_len * chunk_count;

    let poly = random_poly(num_shards);

    // Convert the polynomial to segments
    let kzg = KZG::default_embedded();
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
    let recovered_segments = recovery_row_from_segments(&random_segments, &kzg, chunk_count).unwrap();

    // Verify if the recovered segments are the same as the original segments
    for i in 0..chunk_count {
        assert_eq!(recovered_segments[i], segments[i]);
    }

    // Remove one segment from random_segments
    let mut segments_err = random_segments.clone();
    segments_err.remove(0);
    // Recover segments, it should fail due to an incorrect number of segments
    let result = recovery_row_from_segments(&segments_err, &kzg, chunk_count);

    // Verify if it fails
    assert!(result.is_err());

    // Modify one y value in random_segments
    let mut segments_err = random_segments.clone();
    segments_err[0].position.y = 3;
    // Recover segments, it should fail due to incorrect x values
    let result = recovery_row_from_segments(&segments_err, &kzg, chunk_count);
    // Verify if it fails
    assert!(result.is_err());

    // segment size and chunk_count must be a power of two
    let result = recovery_row_from_segments(&segments_err, &kzg, chunk_count + 1);
    assert!(result.is_err());

    // remove one of the segment.data
    let mut segments_err = random_segments.clone();
    segments_err[0].content.data.remove(0);
    // Recover segments, it should fail due to incorrect segment.data length
    let result = recovery_row_from_segments(&segments_err, &kzg, chunk_count);
    // Verify if it fails
    assert!(result.is_err());

    // segments is not enough
    let mut segments_err = random_segments.clone();
    segments_err.remove(0);
    // Recover segments, it should fail due to incorrect segment.data length
    let result = recovery_row_from_segments(&segments_err, &kzg, chunk_count);
    // Verify if it fails
    assert!(result.is_err());
    
}

#[test]
fn test_proof_multi() {
    let chunk_len: usize = 16;
    let chunk_count: usize = 4;
    let num_shards = chunk_len * chunk_count;

    let kzg = KZG::default_embedded();

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
        // The ys from the extended coefficients
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

// TODO Modify the way data is interleaved
#[test]
fn test_extend_and_commit_multi() {
    let chunk_len: usize = 16;
    let chunk_count: usize = 4;
    let num_shards = chunk_len * chunk_count;

    let kzg = KZG::default_embedded();

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
        // The ys from the extended coefficients
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

fn extend_returns_err_case(
    num_shards: usize,
) {
    let kzg = KZG::default_embedded();

    let evens = (0..num_shards)
        .map(|_| rand::random::<[u8; 31]>())
        .map(BlsScalar::from)
        .collect::<Vec<_>>();

    let result = extend(&kzg.get_fs(), &evens);
    assert!(result.is_err());
}

#[test]
fn test_extend_returns_err() {
    extend_returns_err_case(5);
    extend_returns_err_case(0);
    extend_returns_err_case(321);
}

#[test]
fn test_extend_fs_g1() {
    let kzg = KZG::default_embedded();
    let mut commits: Vec<KZGCommitment> = Vec::new();
    for _rep in 0..4 {
        commits.push(KZGCommitment(FsG1::rand()));
    }
    let extended_commits = extend_fs_g1(kzg.get_fs(), &commits).unwrap();
    assert!(extended_commits.len() == 8);

    for i in 0..4 {
        assert_eq!(extended_commits[i * 2], commits[i]);
    }

    commits.push(KZGCommitment(FsG1::rand()));
    let result = extend_fs_g1(kzg.get_fs(), &commits);
    assert!(result.is_err());

    // Test the empty case
    let empty_commits: Vec<KZGCommitment> = Vec::new();
    let result = extend_fs_g1(kzg.get_fs(), &empty_commits);
    assert!(result.is_err());
}

#[test]
fn test_extend_segments_col() {
    // Build multiple polynomials with random coefficients
    let chunk_len: usize = 16;
    let chunk_count: usize = 4;
    let num_shards = chunk_len * chunk_count;
    let k: usize = 4;
    let polys = (0..k).map(|_| random_poly(num_shards)).collect::<Vec<_>>();
    // Commit to all polynomials
    let kzg = KZG::default_embedded();
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

    // Modify a single x value in the column
    let mut modified_col = extended_col.clone();
    modified_col[0].position.x = 3;

    // Extend the column, it should fail due to incorrect x values
    let extended_col_err = extend_segments_col(kzg.get_fs(), &modified_col);
    assert!(extended_col_err.is_err());

    // Add 3 random segments to the column
    for _ in 0..3 {
        let data = (0..chunk_len)
            .map(|_| rand::random::<[u8; 31]>())
            .map(BlsScalar::from)
            .collect::<Vec<_>>();
        let proof = KZGProof(FsG1::rand());
        let segment_data = SegmentData { data, proof };
        let position = Position { x: 0, y: 0 };
        modified_col.push(Segment { position, content: segment_data });
    }

    // Extend the column, it should fail due to an incorrect number of segments
    let extended_col_err = extend_segments_col(kzg.get_fs(), &modified_col);
    assert!(extended_col_err.is_err());

    // Modify a single y value in the column
    let mut extended_col_err = extended_col.clone();
    extended_col_err[0].position.y = 3;

    // Extend the column, it should fail due to incorrect y values
    let extended_col = extend_segments_col(kzg.get_fs(), &modified_col);
    assert!(extended_col.is_err());

}

#[test]
fn test_bytes_to_segments_round_trip() {
    // Build random bytes
    let chunk_len: usize = 16;
    let chunk_count: usize = 4;
    let num_shards = chunk_len * chunk_count;
    let bytes_len = num_shards * 31;
    let bytes = random_bytes(bytes_len);

    // Convert bytes to blob
    let blob = Blob::try_from_bytes_pad(&bytes, bytes_len).unwrap();
    // Convert blob to segments
    let kzg = KZG::default_embedded();
    let poly = blob.to_poly();
    let commitment = blob.commit(&kzg).unwrap();
    let segments = poly_to_segment_vec(&poly, &kzg, 0, chunk_len).unwrap();
    // Verify all segments are correct
    for i in 0..chunk_count {
        let verify = segments[i].verify(&kzg, &commitment, chunk_count).unwrap();
        assert!(verify);
    }
    // Convert all segments to segment datas
    let segment_datas = segments
        .iter()
        .map(|segment| Some(segment.content.clone()))
        .collect::<Vec<_>>();
    // Convert segment datas to row
    let mut row = segment_datas_to_row(&segment_datas, chunk_len);
    row[0] = None;
    // Reverse row
    reverse_bit_order(&mut row);
    let poly2 = recover_poly(&kzg.get_fs(), &row).unwrap();
    // Convert the polynomial to blob
    let blob2 = poly2.to_blob();
    // Verify if blob is correct
    assert_eq!(blob, blob2);
    // Convert the blob to bytes
    let bytes2 = blob.to_bytes();
    // Verify if bytes are the same as the original ones
    assert_eq!(bytes, bytes2);
}

#[test]
fn test_bytes_vec_to_blobs() {
    // Generate an array representing the lengths of bytes
    let bytes_lens: Vec<usize> = vec![20 * 31, 10 * 31 + 7];
    let field_elements_per_blob: usize = 4;
    let bytes_per_blob: usize = 31 * field_elements_per_blob;

    let bytes_in_blob_lens: Vec<usize> = bytes_lens
        .iter()
        .flat_map(|&x| {
            let divided = x / bytes_per_blob;
            let remainder = x % bytes_per_blob;
            let mut new_vec = vec![bytes_per_blob; divided];
            if remainder > 0 {
                new_vec.push(remainder);
            }
            new_vec
        })
        .collect();
    // Generate random Vec<bytes> based on the lengths and convert them to blobs
    let bytes_vec: Vec<Vec<u8>> = bytes_lens.iter().map(|&len| random_bytes(len)).collect();
    // Convert each Vec<bytes> to blobs
    let blobs: Vec<Blob> = bytes_vec_to_blobs(&bytes_vec, field_elements_per_blob).unwrap();
    assert!(blobs.len() == bytes_in_blob_lens.len());
    // Convert all blobs back to bytes
    let bytes_vec2: Vec<Vec<u8>> = blobs
        .iter()
        .enumerate()
        .map(|(i, blob)| blob.to_bytes_by_len(bytes_in_blob_lens[i]))
        .collect();
    bytes_vec.iter().fold(0, |acc, bytes| {
        let mut index: usize = 0;
        bytes.chunks(bytes_per_blob).enumerate().for_each(|(i, chunk)| {
            index += 1;
            assert_eq!(chunk, &bytes_vec2[acc + i]);
        });
        acc + index
    });
}

fn bytes_vec_to_blobs_returns_err_case(bytes_lens: Vec<usize>, field_elements_per_blob: usize) {
    let bytes_vec: Vec<Vec<u8>> = bytes_lens.iter().map(|&len| random_bytes(len)).collect();
    let result = bytes_vec_to_blobs(&bytes_vec, field_elements_per_blob);
    assert!(result.is_err());
}

#[test]
fn test_bytes_vec_to_blobs_returns_err() {
    bytes_vec_to_blobs_returns_err_case(vec![20 * 31, 0], 4);
    bytes_vec_to_blobs_returns_err_case(vec![0], 4);
    bytes_vec_to_blobs_returns_err_case(vec![20 * 31, 0, 20 * 31], 4);
    bytes_vec_to_blobs_returns_err_case(vec![20 * 31], 3);
    bytes_vec_to_blobs_returns_err_case(vec![20 * 31], 0);
}
