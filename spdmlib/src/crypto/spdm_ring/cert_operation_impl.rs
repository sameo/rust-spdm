// Copyright (c) 2021 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;
use core::convert::TryFrom;

use crate::crypto::SpdmCertOperation;
use crate::error::{spdm_result_err, SpdmResult};
use ring::io::der;

pub static DEFAULT: SpdmCertOperation = SpdmCertOperation {
    get_cert_from_cert_chain_cb: get_cert_from_cert_chain,
    verify_cert_chain_cb: verify_cert_chain,
};

fn get_cert_from_cert_chain(cert_chain: &[u8], index: isize) -> SpdmResult<(usize, usize)> {
    let mut offset = 0usize;
    let mut this_index = 0isize;
    loop {
        if cert_chain[offset..].len() < 4 || offset > cert_chain.len() {
            return spdm_result_err!(EINVAL);
        }
        if cert_chain[offset] != 0x30 || cert_chain[offset + 1] != 0x82 {
            return spdm_result_err!(EINVAL);
        }
        let this_cert_len =
            ((cert_chain[offset + 2] as usize) << 8) + (cert_chain[offset + 3] as usize) + 4;
        //debug!("this_cert_len - 0x{:04x?}\n", this_cert_len);
        if this_index == index {
            // return the this one
            return Ok((offset, offset + this_cert_len));
        }
        this_index += 1;
        if (offset + this_cert_len == cert_chain.len()) && (index == -1) {
            // return the last one
            return Ok((offset, offset + this_cert_len));
        }
        offset += this_cert_len;
    }
}

fn verify_cert_chain(cert_chain: &[u8]) -> SpdmResult {
    // TBD
    static EKU_SPDM_RESPONDER_AUTH: &[u8] = &[40 + 3, 6, 1, 5, 5, 7, 3, 1];

    static ALL_SIGALGS: &[&webpki::SignatureAlgorithm] = &[
        &webpki::RSA_PKCS1_2048_8192_SHA256,
        &webpki::RSA_PKCS1_2048_8192_SHA384,
        &webpki::RSA_PKCS1_2048_8192_SHA512,
        &webpki::ECDSA_P256_SHA256,
        &webpki::ECDSA_P256_SHA384,
        &webpki::ECDSA_P384_SHA256,
        &webpki::ECDSA_P384_SHA384,
    ];

    let certs_der = untrusted::Input::from(cert_chain);
    let reader = &mut untrusted::Reader::new(certs_der);

    let mut certs = Vec::new();
    loop {
        let start = reader.mark();
        match der::expect_tag_and_get_value(reader, der::Tag::Sequence) {
            Ok(_) => {
                let end = reader.mark();
                let cert = reader
                    .get_input_between_marks(start, end)
                    .map_err(|_| spdm_err!(EINVAL))?;
                certs.push(cert.as_slice_less_safe())
            }
            Err(_) => break,
        }
    }
    let certs_len = certs.len();

    let (ca, inters, ee): (&[u8], &[&[u8]], &[u8]) = match certs_len {
        0 => return spdm_result_err!(EINVAL),
        1 => (certs[0], &[], certs[0]),
        2 => (certs[0], &[], certs[1]),
        n => (certs[0], &certs[1..(n - 1)], certs[n - 1]),
    };

    let anchors = if let Ok(ta) = webpki::TrustAnchor::try_from_cert_der(ca) {
        vec![ta]
    } else {
        return spdm_result_err!(ESEC);
    };

    #[cfg(any(target_os = "uefi", target_os = "none"))]
    let timestamp = uefi_time::get_rtc_time() as u64;
    #[cfg(not(any(target_os = "uefi", target_os = "none")))]
    let timestamp = {
        extern crate std;
        if let Ok(ds) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            ds.as_secs()
        } else {
            return spdm_result_err!(EDEV);
        }
    };
    let time = webpki::Time::from_seconds_since_unix_epoch(timestamp);

    let cert = if let Ok(eec) = webpki::EndEntityCert::try_from(ee) {
        eec
    } else {
        return spdm_result_err!(ESEC);
    };

    // we cannot call verify_is_valid_tls_server_cert because it will check verify_cert::EKU_SERVER_AUTH.
    if cert
        .verify_cert_chain_with_eku(
            EKU_SPDM_RESPONDER_AUTH,
            ALL_SIGALGS,
            &anchors,
            inters,
            time,
            0,
        )
        .is_ok()
    {
        info!("Cert verification Pass\n");
        Ok(())
    } else {
        error!("Cert verification Fail\n");
        spdm_result_err!(EFAULT)
    }
}
#[cfg(all(test,))]
mod tests {
    use super::*;

    #[test]
    fn test_case0_cert_from_cert_chain() {
        let cert_chain = &include_bytes!("public_cert.der")[..];
        let status = get_cert_from_cert_chain(cert_chain, -1).is_ok();
        assert!(status);
    }

    #[test]
    fn test_case1_cert_from_cert_chain() {
        let cert_chain = &include_bytes!("public_cert.der")[..];
        let status = get_cert_from_cert_chain(cert_chain, 0).is_ok();
        assert!(status);
    }
    #[test]
    fn test_case2_cert_from_cert_chain() {
        let cert_chain = &include_bytes!("public_cert.der")[..];
        let status = get_cert_from_cert_chain(cert_chain, 1).is_ok();
        assert!(status);
    }
    #[test]
    fn test_case3_cert_from_cert_chain() {
        let cert_chain = &mut [0x1u8; 4096];
        cert_chain[0] = 0x00;
        cert_chain[1] = 0x00;
        let status = get_cert_from_cert_chain(cert_chain, 0).is_err();
        assert!(status);
    }
    #[test]
    fn test_case4_cert_from_cert_chain() {
        let cert_chain = &mut [0x11u8; 3];
        let status = get_cert_from_cert_chain(cert_chain, 0).is_err();
        assert!(status);
    }
    #[test]
    fn test_case5_cert_from_cert_chain() {
        let cert_chain = &include_bytes!("public_cert.der")[..];
        let status = get_cert_from_cert_chain(cert_chain, -1).is_ok();
        assert!(status);

        let status = verify_cert_chain(cert_chain).is_ok();
        assert!(status);
    }

    /// verfiy cert chain
    #[test]
    fn test_verify_cert_chain_case1() {
        let bundle_certs_der =
            &include_bytes!("../../../../test_key/crypto_chains/ca_selfsigned.crt.der")[..];
        assert!(verify_cert_chain(bundle_certs_der).is_ok());

        let bundle_certs_der =
            &include_bytes!("../../../../test_key/crypto_chains/bundle_two_level_cert.der")[..];
        assert!(verify_cert_chain(bundle_certs_der).is_ok());

        let bundle_certs_der =
            &include_bytes!("../../../../test_key/EcP384/bundle_requester.certchain.der")[..];
        assert!(verify_cert_chain(bundle_certs_der).is_ok());

        let bundle_certs_der =
            &include_bytes!("../../../../test_key/crypto_chains/bundle_cert.der")[..];
        assert!(verify_cert_chain(bundle_certs_der).is_ok())
    }
}
