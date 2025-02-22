// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use codec::enum_builder;
use codec::{Codec, Reader, Writer};
use spdmlib::common::SpdmTransportEncap;
use spdmlib::error::SpdmResult;
use spdmlib::{spdm_err, spdm_result_err};

enum_builder! {
    @U8
    EnumName: MctpMessageType;
    EnumVal{
        MctpMessageTypeMctpControl => 0x00,
        MctpMessageTypePldm => 0x01,
        MctpMessageTypeNcsi => 0x02,
        MctpMessageTypeEthernet => 0x03,
        MctpMessageTypeNvme => 0x04,
        MctpMessageTypeSpdm => 0x05,
        MctpMessageTypeSecuredMctp => 0x06,
        MctpMessageTypeVendorDefinedPci => 0x7E,
        MctpMessageTypeVendorDefinedIana => 0x7F
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct MctpMessageHeader {
    pub r#type: MctpMessageType,
}

impl Codec for MctpMessageHeader {
    fn encode(&self, bytes: &mut Writer) {
        self.r#type.encode(bytes);
    }

    fn read(r: &mut Reader) -> Option<MctpMessageHeader> {
        let r#type = MctpMessageType::read(r)?;
        Some(MctpMessageHeader { r#type })
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct MctpTransportEncap {}

impl SpdmTransportEncap for MctpTransportEncap {
    fn encap(
        &mut self,
        spdm_buffer: &[u8],
        transport_buffer: &mut [u8],
        secured_message: bool,
    ) -> SpdmResult<usize> {
        let payload_len = spdm_buffer.len();
        let mut writer = Writer::init(&mut *transport_buffer);
        let mctp_header = MctpMessageHeader {
            r#type: if secured_message {
                MctpMessageType::MctpMessageTypeSecuredMctp
            } else {
                MctpMessageType::MctpMessageTypeSpdm
            },
        };
        mctp_header.encode(&mut writer);
        let header_size = writer.used();
        if transport_buffer.len() < header_size + payload_len {
            return spdm_result_err!(EINVAL);
        }
        transport_buffer[header_size..(header_size + payload_len)].copy_from_slice(spdm_buffer);
        Ok(header_size + payload_len)
    }

    fn decap(
        &mut self,
        transport_buffer: &[u8],
        spdm_buffer: &mut [u8],
    ) -> SpdmResult<(usize, bool)> {
        let mut reader = Reader::init(transport_buffer);
        let secured_message;
        match MctpMessageHeader::read(&mut reader) {
            Some(mctp_header) => match mctp_header.r#type {
                MctpMessageType::MctpMessageTypeSpdm => {
                    secured_message = false;
                }
                MctpMessageType::MctpMessageTypeSecuredMctp => {
                    secured_message = true;
                }
                _ => return spdm_result_err!(EINVAL),
            },
            None => return spdm_result_err!(EIO),
        }
        let header_size = reader.used();
        let payload_size = transport_buffer.len() - header_size;
        if spdm_buffer.len() < payload_size {
            return spdm_result_err!(EINVAL);
        }
        let payload = &transport_buffer[header_size..];
        spdm_buffer[..payload_size].copy_from_slice(payload);
        Ok((payload_size, secured_message))
    }

    fn encap_app(
        &mut self,
        spdm_buffer: &[u8],
        app_buffer: &mut [u8],
        is_app_message: bool,
    ) -> SpdmResult<usize> {
        let payload_len = spdm_buffer.len();
        let mut writer = Writer::init(&mut *app_buffer);
        let mctp_header = if is_app_message {
            MctpMessageHeader {
                r#type: MctpMessageType::MctpMessageTypePldm,
            }
        } else {
            MctpMessageHeader {
                r#type: MctpMessageType::MctpMessageTypeSpdm,
            }
        };
        mctp_header.encode(&mut writer);
        let header_size = writer.used();
        if app_buffer.len() < header_size + payload_len {
            return spdm_result_err!(EINVAL);
        }
        app_buffer[header_size..(header_size + payload_len)].copy_from_slice(spdm_buffer);
        Ok(header_size + payload_len)
    }

    fn decap_app(
        &mut self,
        app_buffer: &[u8],
        spdm_buffer: &mut [u8],
    ) -> SpdmResult<(usize, bool)> {
        let mut reader = Reader::init(app_buffer);
        let mut is_app_mesaage = false;
        match MctpMessageHeader::read(&mut reader) {
            Some(mctp_header) => match mctp_header.r#type {
                MctpMessageType::MctpMessageTypeSpdm => {}
                MctpMessageType::MctpMessageTypePldm => {
                    is_app_mesaage = true;
                }
                _ => return spdm_result_err!(EINVAL),
            },
            None => return spdm_result_err!(EIO),
        }
        let header_size = reader.used();
        let payload_size = app_buffer.len() - header_size;
        if spdm_buffer.len() < payload_size {
            return spdm_result_err!(EINVAL);
        }
        let payload = &app_buffer[header_size..];
        spdm_buffer[..payload_size].copy_from_slice(payload);
        Ok((payload_size, is_app_mesaage))
    }

    fn get_sequence_number_count(&mut self) -> u8 {
        2
    }
    fn get_max_random_count(&mut self) -> u16 {
        32
    }
}

#[cfg(all(test,))]
mod tests {
    use spdmlib::config;

    use super::*;

    #[test]
    fn test_case0_mctpmessageheader() {
        let u8_slice = &mut [0u8; 1];
        let mut writer = Writer::init(u8_slice);
        let value = MctpMessageHeader {
            r#type: MctpMessageType::MctpMessageTypeMctpControl,
        };
        value.encode(&mut writer);
        let mut reader = Reader::init(u8_slice);
        assert_eq!(1, reader.left());
        let mctp_message_header = MctpMessageHeader::read(&mut reader).unwrap();
        assert_eq!(0, reader.left());
        assert_eq!(
            mctp_message_header.r#type,
            MctpMessageType::MctpMessageTypeMctpControl
        );
    }
    #[test]
    fn test_case0_encap() {
        let mut mctp_transport_encap = MctpTransportEncap {};
        let mut transport_buffer = [100u8; config::DATA_TRANSFER_SIZE];
        let spdm_buffer = [100u8; config::MAX_SPDM_MESSAGE_BUFFER_SIZE];

        let status = mctp_transport_encap
            .encap(&spdm_buffer, &mut transport_buffer, false)
            .is_ok();
        assert!(status);

        let status = mctp_transport_encap
            .encap(&spdm_buffer, &mut transport_buffer, true)
            .is_ok();
        assert!(status);

        let mut transport_buffer = [100u8; config::DATA_TRANSFER_SIZE];
        let spdm_buffer = [100u8; config::DATA_TRANSFER_SIZE];
        let status = mctp_transport_encap
            .encap(&spdm_buffer, &mut transport_buffer, true)
            .is_err();
        assert!(status);
    }
    #[test]
    fn test_case0_decap() {
        let mut mctp_transport_encap = MctpTransportEncap {};

        let mut spdm_buffer = [100u8; config::DATA_TRANSFER_SIZE];

        let transport_buffer = &mut [0u8; 10];

        let status = mctp_transport_encap
            .decap(transport_buffer, &mut spdm_buffer)
            .is_err();
        assert!(status);

        let mut writer = Writer::init(transport_buffer);
        let value = MctpMessageHeader {
            r#type: MctpMessageType::MctpMessageTypeSpdm,
        };
        value.encode(&mut writer);

        let status = mctp_transport_encap
            .decap(transport_buffer, &mut spdm_buffer)
            .is_ok();
        assert!(status);

        let transport_buffer = &mut [0u8; 2];
        let mut writer = Writer::init(transport_buffer);
        let value = MctpMessageHeader {
            r#type: MctpMessageType::MctpMessageTypeSecuredMctp,
        };
        value.encode(&mut writer);

        let status = mctp_transport_encap
            .decap(transport_buffer, &mut spdm_buffer)
            .is_ok();
        assert!(status);
    }
    #[test]
    fn test_case0_encap_app() {
        let mut mctp_transport_encap = MctpTransportEncap {};
        let mut app_buffer = [0u8; 100];
        let spdm_buffer = [0u8; 10];

        let status = mctp_transport_encap
            .encap_app(&spdm_buffer, &mut app_buffer, false)
            .is_ok();
        assert!(status);

        let spdm_buffer = [100u8; 1024];

        let status = mctp_transport_encap
            .encap_app(&spdm_buffer, &mut app_buffer, false)
            .is_err();
        assert!(status);
    }
    #[test]
    fn test_case0_decap_app() {
        let mut mctp_transport_encap = MctpTransportEncap {};

        let mut spdm_buffer = [100u8; config::DATA_TRANSFER_SIZE];

        let transport_buffer = &mut [0u8; 10];

        let status = mctp_transport_encap
            .decap_app(transport_buffer, &mut spdm_buffer)
            .is_err();
        assert!(status);

        let mut writer = Writer::init(transport_buffer);
        let value = MctpMessageHeader {
            r#type: MctpMessageType::MctpMessageTypeSpdm,
        };
        value.encode(&mut writer);

        let status = mctp_transport_encap
            .decap_app(transport_buffer, &mut spdm_buffer)
            .is_ok();
        assert!(status);
    }
    #[test]
    fn test_case0_get_sequence_number_count() {
        let mut mctp_transport_encap = MctpTransportEncap {};
        assert_eq!(mctp_transport_encap.get_sequence_number_count(), 2);
    }
    #[test]
    fn test_case0_get_max_random_count() {
        let mut mctp_transport_encap = MctpTransportEncap {};
        assert_eq!(mctp_transport_encap.get_max_random_count(), 32);
    }
}
