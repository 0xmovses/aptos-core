// Copyright © Aptos Foundation

use crate::vuf::VUF;
use anyhow::{anyhow, ensure};
use ark_bls12_381::{Bls12_381, Fq12, Fr, G1Affine, G2Affine, G2Projective};
use ark_ec::{
    hashing::HashToCurve, pairing::Pairing, short_weierstrass::Projective, AffineRepr, CurveGroup,
    Group,
};
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};
use std::ops::Mul;

pub struct Bls12381G1Bls {}

pub static DST: &[u8] = b"APTOS_PEPPER_SERVICE_BLS12381_VUF_DST";

impl Bls12381G1Bls {
    fn hash_to_g1(input: &[u8]) -> G1Affine {
        let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
            Projective<ark_bls12_381::g1::Config>,
            ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
            ark_ec::hashing::curve_maps::wb::WBMap<ark_bls12_381::g1::Config>,
        >::new(DST)
        .unwrap();
        mapper.hash(input).unwrap()
    }
}

pub const SCHEME_NAME: &str = "BLS12381_G1_BLS";

impl VUF for Bls12381G1Bls {
    type PrivateKey = Fr;
    type PublicKey = G2Projective;

    fn scheme_name() -> String {
        SCHEME_NAME.to_string()
    }

    fn setup<R: CryptoRng + RngCore>(rng: &mut R) -> (Self::PrivateKey, Self::PublicKey) {
        let sk = Fr::rand(rng);
        let pk = G2Affine::generator() * sk;
        (sk, pk)
    }

    fn pk_from_sk(sk: &Fr) -> anyhow::Result<G2Projective> {
        Ok(G2Projective::generator() * sk)
    }

    fn eval(sk: &Fr, input: &[u8]) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        let input_g1 = Self::hash_to_g1(input);
        let output_g1 = input_g1.mul(sk).into_affine();
        let mut output_bytes = vec![];
        output_g1
            .serialize_compressed(&mut output_bytes)
            .map_err(|e| {
                anyhow!("Bls12381G1Bls::eval failed with output serialization error: {e}")
            })?;
        Ok((output_bytes, vec![]))
    }

    fn verify(
        pk_g2: &G2Projective,
        input: &[u8],
        output: &[u8],
        proof: &[u8],
    ) -> anyhow::Result<()> {
        ensure!(
            proof.is_empty(),
            "Bls12381G1Bls::verify failed with proof deserialization error"
        );
        let input_g1 = Self::hash_to_g1(input);
        let output_g1 = G1Affine::deserialize_compressed(output).map_err(|e| {
            anyhow!("Bls12381G1Bls::verify failed with output deserialization error: {e}")
        })?;
        ensure!(
            Fq12::ONE
                == Bls12_381::multi_pairing([-output_g1, input_g1], [
                    G2Affine::generator(),
                    (*pk_g2).into_affine()
                ])
                .0,
            "Bls12381G1Bls::verify failed with final check failure"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::vuf::{bls12381_g1_bls::Bls12381G1Bls, VUF};

    #[test]
    fn gen_eval_verify() {
        let mut rng = ark_std::rand::thread_rng();
        let (sk, pk) = Bls12381G1Bls::setup(&mut rng);
        let pk_another = Bls12381G1Bls::pk_from_sk(&sk).unwrap();
        assert_eq!(pk_another, pk);
        let input: &[u8] = b"hello world again and again and again and again and again and again";
        let (output, proof) = Bls12381G1Bls::eval(&sk, input).unwrap();
        Bls12381G1Bls::verify(&pk, input, &output, &proof).unwrap();
        println!("output={:?}", output);
    }
}
