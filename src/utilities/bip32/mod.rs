use curv::arithmetic::traits::Converter;
use curv::cryptographic_primitives::hashing::HmacExt;
use curv::elliptic::curves::Point;
use curv::elliptic::curves::Scalar;
use curv::elliptic::curves::Secp256k1;
use curv::{
    arithmetic::{BasicOps, One},
    BigInt,
};
use sha2::Sha512;

use hmac::{Hmac, NewMac};

pub fn get_hd_key(
    y_sum: Point<Secp256k1>,
    path_vector: Vec<BigInt>,
) -> (Point<Secp256k1>, Scalar<Secp256k1>) {
    // generate a random but shared chain code, this will do
    let chain_code = Point::<Secp256k1>::generator().to_point();
    // derive a new pubkey and LR sequence, y_sum becomes a new child pub key
    let (y_sum_child, f_l_new, _cc_new) = hd_key(
        path_vector,
        y_sum,
        &BigInt::from_bytes(chain_code.to_bytes(true).as_ref()),
    );

    (y_sum_child, f_l_new)
}

pub fn hd_key(
    mut location_in_hir: Vec<BigInt>,
    pubkey: Point<Secp256k1>,
    chain_code_bi: &BigInt,
) -> (Point<Secp256k1>, Scalar<Secp256k1>, Point<Secp256k1>) {
    let mask = BigInt::from(2).pow(256) - BigInt::one();
    // calc first element:
    let first = location_in_hir.remove(0);
    let pub_key_bi = &BigInt::from_bytes(pubkey.to_bytes(true).as_ref());

    let f = Hmac::<Sha512>::new_from_slice(&chain_code_bi.to_bytes())
        .unwrap()
        .chain_bigint(pub_key_bi)
        .chain_bigint(&first)
        .result_bigint();
    let f_l = &f >> 256;
    let f_r = &f & &mask;
    let f_l_fe = Scalar::<Secp256k1>::from(&f_l);
    let f_r_fe = Scalar::<Secp256k1>::from(&f_r);

    let bn_to_slice = BigInt::to_bytes(chain_code_bi);
    let chain_code = Point::from_bytes(&bn_to_slice).unwrap() * &f_r_fe;
    let g = Point::generator();
    let pub_key = pubkey + g * &f_l_fe;

    let (public_key_new_child, f_l_new, cc_new) =
        location_in_hir
            .iter()
            .fold((pub_key, f_l_fe, chain_code), |acc, index| {
                let pub_key_bi = &BigInt::from_bytes(acc.0.to_bytes(true).as_ref());

                let f = Hmac::<Sha512>::new_from_slice(&acc.2.to_bytes(true))
                    .unwrap()
                    .chain_bigint(pub_key_bi)
                    .chain_bigint(index)
                    .result_bigint();

                let f_l = &f >> 256;
                let f_r = &f & &mask;
                let f_l_fe = Scalar::<Secp256k1>::from(&f_l);
                let f_r_fe = Scalar::<Secp256k1>::from(&f_r);

                (acc.0 + g * &f_l_fe, f_l_fe + &acc.1, &acc.2 * &f_r_fe)
            });
    (public_key_new_child, f_l_new, cc_new)
}

#[test]
fn derive_test() {
    let original_x =
        BigInt::from_hex("d6f3c325eb3fda7061983141278484c0dd452a6702fd537b89c09ddf2b6f3238")
            .unwrap();
    let original_y =
        BigInt::from_hex("4e12adae75c29b29cc094fd3d94aa401ea646104f0d1ae3c59f710ec92640e21")
            .unwrap();

    let original_public_key = Point::<Secp256k1>::from_coords(&original_x, &original_y).unwrap();

    let path = "1/2/3";
    let path_vector: Vec<BigInt> = path
        .split('/')
        .map(|s| BigInt::from_str_radix(s.trim(), 10).unwrap())
        .collect();

    let expected_pubkey_x = "e891363052c09185814e92ce7a1a1946631dc53d058a01176fcf27a66b5674c2";
    let expected_pubkey_y = "cfbe0a84b7f7c49b5bb2a48999a761fc6c5dd6526aa79a58d4029865ef7d4a17";
    let (public_key_child, _f_l_new) = get_hd_key(original_public_key, path_vector);

    assert_eq!(
        public_key_child.x_coord().unwrap().to_hex(),
        expected_pubkey_x
    );
    assert_eq!(
        public_key_child.y_coord().unwrap().to_hex(),
        expected_pubkey_y
    );
}
