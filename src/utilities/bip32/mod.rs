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
    //    println!("chain code {:?}", chain_code);
    // derive a new pubkey and LR sequence, y_sum becomes a new child pub key
    let (y_sum_child, f_l_new, _cc_new) = hd_key(
        path_vector,
        y_sum,
        &BigInt::from_bytes(chain_code.to_bytes(true).as_ref()),
    );
    let y_sum = y_sum_child.clone();
    //    println!("New public key: {:?}", &y_sum);
    //    println!("Public key X: {:?}", &y_sum.x_coor());
    //    println!("Public key Y: {:?}", &y_sum.y_coor());
    (y_sum, f_l_new)
}

pub fn hd_key(
    mut location_in_hir: Vec<BigInt>,
    pubkey: Point<Secp256k1>,
    chain_code_bi: &BigInt,
) -> (Point<Secp256k1>, Scalar<Secp256k1>, Point<Secp256k1>) {
    let mask = BigInt::from(2).pow(256) - BigInt::one();
    // let public_key = self.public.q.clone();

    // calc first element:
    let first = location_in_hir.remove(0);
    let pub_key_bi = &BigInt::from_bytes(pubkey.to_bytes(true).as_ref());

    let f = Hmac::<Sha512>::new_from_slice(&chain_code_bi.to_bytes())
        .unwrap()
        .chain_bigint(&pub_key_bi)
        .chain_bigint(&first)
        .result_bigint();
    // let f = hmac_sha512::HMacSha512::create_hmac(&chain_code_bi, &[&pub_key_bi, &first]);
    let f_l = &f >> 256;
    let f_r = &f & &mask;
    let f_l_fe = Scalar::<Secp256k1>::from(&f_l);
    let f_r_fe = Scalar::<Secp256k1>::from(&f_r);

    let bn_to_slice = BigInt::to_bytes(chain_code_bi);
    let chain_code = Point::from_bytes(&bn_to_slice[1..33]).unwrap() * &f_r_fe;
    let g = Point::generator();
    let pub_key = pubkey + g * &f_l_fe;

    let (public_key_new_child, f_l_new, cc_new) =
        location_in_hir
            .iter()
            .fold((pub_key, f_l_fe, chain_code), |acc, index| {
                let pub_key_bi = &BigInt::from_bytes(acc.0.to_bytes(true).as_ref());
                // let f = hmac_sha512::HMacSha512::create_hmac(
                //     &acc.2.bytes_compressed_to_big_int(),
                //     &[&pub_key_bi, index],
                // );

                let f = Hmac::<Sha512>::new_from_slice(&acc.2.to_bytes(true))
                    .unwrap()
                    .chain_bigint(&pub_key_bi)
                    .chain_bigint(&index)
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
    let base_u = Scalar::<Secp256k1>::random();
    let base_y = Point::<Secp256k1>::generator() * &base_u;
    let path = "44/60/0";
    let path_vector: Vec<BigInt> = path
        .split('/')
        .map(|s| BigInt::from_str_radix(s.trim(), 10).unwrap())
        .collect();

    let (u, y) = get_hd_key(base_y, path_vector);
}
