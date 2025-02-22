// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use crate::common;
use crate::common::spdm_codec::SpdmCodec;
use codec::enum_builder;
use codec::{Codec, Reader, Writer};

enum_builder! {
    @U8
    EnumName: SpdmKeyUpdateOperation;
    EnumVal{
        SpdmUpdateSingleKey => 0x1,
        SpdmUpdateAllKeys => 0x2,
        SpdmVerifyNewKey => 0x3
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpdmKeyUpdateRequestPayload {
    pub key_update_operation: SpdmKeyUpdateOperation,
    pub tag: u8,
}

impl SpdmCodec for SpdmKeyUpdateRequestPayload {
    fn spdm_encode(&self, _context: &mut common::SpdmContext, bytes: &mut Writer) {
        self.key_update_operation.encode(bytes); // param1
        self.tag.encode(bytes); // param2
    }

    fn spdm_read(
        _context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmKeyUpdateRequestPayload> {
        let key_update_operation = SpdmKeyUpdateOperation::read(r)?; // param1
        let tag = u8::read(r)?; // param2

        Some(SpdmKeyUpdateRequestPayload {
            key_update_operation,
            tag,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpdmKeyUpdateResponsePayload {
    pub key_update_operation: SpdmKeyUpdateOperation,
    pub tag: u8,
}

impl SpdmCodec for SpdmKeyUpdateResponsePayload {
    fn spdm_encode(&self, _context: &mut common::SpdmContext, bytes: &mut Writer) {
        self.key_update_operation.encode(bytes); // param1
        self.tag.encode(bytes); // param2
    }

    fn spdm_read(
        _context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmKeyUpdateResponsePayload> {
        let key_update_operation = SpdmKeyUpdateOperation::read(r)?; // param1
        let tag = u8::read(r)?; // param2

        Some(SpdmKeyUpdateResponsePayload {
            key_update_operation,
            tag,
        })
    }
}

#[cfg(all(test,))]
#[path = "mod_test.common.inc.rs"]
mod testlib;

#[cfg(all(test,))]
mod tests {
    use super::*;
    use crate::common::{SpdmConfigInfo, SpdmContext, SpdmProvisionInfo};
    use testlib::{create_spdm_context, DeviceIO, TransportEncap};

    #[test]
    fn test_case0_spdm_key_update_request_payload() {
        let u8_slice = &mut [0u8; 2];
        let mut writer = Writer::init(u8_slice);
        let value = SpdmKeyUpdateRequestPayload {
            key_update_operation: SpdmKeyUpdateOperation::SpdmUpdateAllKeys,
            tag: 100u8,
        };

        create_spdm_context!(context);

        value.spdm_encode(&mut context, &mut writer);
        let mut reader = Reader::init(u8_slice);
        assert_eq!(2, reader.left());
        let key_request_payload =
            SpdmKeyUpdateRequestPayload::spdm_read(&mut context, &mut reader).unwrap();
        assert_eq!(
            key_request_payload.key_update_operation,
            SpdmKeyUpdateOperation::SpdmUpdateAllKeys
        );
        assert_eq!(key_request_payload.tag, 100);
        assert_eq!(0, reader.left());
    }
    #[test]
    fn test_case0_spdm_key_update_response_payload() {
        let u8_slice = &mut [0u8; 2];
        let mut writer = Writer::init(u8_slice);
        let value = SpdmKeyUpdateResponsePayload {
            key_update_operation: SpdmKeyUpdateOperation::SpdmUpdateAllKeys,
            tag: 100u8,
        };

        create_spdm_context!(context);

        value.spdm_encode(&mut context, &mut writer);
        let mut reader = Reader::init(u8_slice);
        assert_eq!(2, reader.left());
        let key_response_payload =
            SpdmKeyUpdateResponsePayload::spdm_read(&mut context, &mut reader).unwrap();
        assert_eq!(
            key_response_payload.key_update_operation,
            SpdmKeyUpdateOperation::SpdmUpdateAllKeys
        );
        assert_eq!(key_response_payload.tag, 100);
        assert_eq!(0, reader.left());
    }
}
