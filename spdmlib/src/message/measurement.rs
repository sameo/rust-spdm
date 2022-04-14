// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use crate::common;
use crate::common::algo::{SpdmMeasurementRecordStructure, SpdmNonceStruct, SpdmSignatureStruct};
use crate::common::opaque::SpdmOpaqueStruct;
use crate::common::spdm_codec::SpdmCodec;
use codec::enum_builder;
use codec::{Codec, Reader, Writer};

use super::SpdmVersion;

pub const MEASUREMENT_RESPONDER_PARAM2_SLOT_ID_MASK: u8 = 0b0000_1111;
pub const MEASUREMENT_RESPONDER_PARAM2_CONTENT_CHANGED_MASK: u8 = 0b0011_0000;
pub const MEASUREMENT_RESPONDER_PARAM2_CONTENT_CHANGED_NOT_SUPPORTED_VALUE: u8 = 0b0000_0000;
pub const MEASUREMENT_RESPONDER_PARAM2_CONTENT_CHANGED_DETECTED_CHANGE_VALUE: u8 = 0b0001_0000;
pub const MEASUREMENT_RESPONDER_PARAM2_CONTENT_CHANGED_NO_CHANGE_VALUE: u8 = 0b0010_0000;

bitflags! {
    #[derive(Default)]
    pub struct SpdmMeasurementeAttributes: u8 {
        const SIGNATURE_REQUESTED = 0b00000001;
        const RAW_BIT_STREAM_REQUESTED = 0b0000_0010;
    }
}

impl Codec for SpdmMeasurementeAttributes {
    fn encode(&self, bytes: &mut Writer) {
        self.bits().encode(bytes);
    }

    fn read(r: &mut Reader) -> Option<SpdmMeasurementeAttributes> {
        let bits = u8::read(r)?;

        SpdmMeasurementeAttributes::from_bits(bits)
    }
}

enum_builder! {
    @U8
    EnumName: SpdmMeasurementOperation;
    EnumVal{
        SpdmMeasurementQueryTotalNumber => 0x0,
        SpdmMeasurementRequestAll => 0xFF
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpdmGetMeasurementsRequestPayload {
    pub measurement_attributes: SpdmMeasurementeAttributes,
    pub measurement_operation: SpdmMeasurementOperation,
    pub nonce: SpdmNonceStruct,
    pub slot_id: u8,
}

impl SpdmCodec for SpdmGetMeasurementsRequestPayload {
    fn spdm_encode(&self, _context: &mut common::SpdmContext, bytes: &mut Writer) {
        self.measurement_attributes.encode(bytes); // param1
        self.measurement_operation.encode(bytes); // param2
        if self
            .measurement_attributes
            .contains(SpdmMeasurementeAttributes::SIGNATURE_REQUESTED)
        {
            self.nonce.encode(bytes);
            self.slot_id.encode(bytes);
        }
    }

    fn spdm_read(
        _context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmGetMeasurementsRequestPayload> {
        let measurement_attributes = SpdmMeasurementeAttributes::read(r)?; // param1
        let measurement_operation = SpdmMeasurementOperation::read(r)?; // param2
        let nonce =
            if measurement_attributes.contains(SpdmMeasurementeAttributes::SIGNATURE_REQUESTED) {
                SpdmNonceStruct::read(r)?
            } else {
                SpdmNonceStruct::default()
            };
        let slot_id =
            if measurement_attributes.contains(SpdmMeasurementeAttributes::SIGNATURE_REQUESTED) {
                u8::read(r)?
            } else {
                0
            };

        Some(SpdmGetMeasurementsRequestPayload {
            measurement_attributes,
            measurement_operation,
            nonce,
            slot_id,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpdmMeasurementsResponsePayload {
    pub number_of_measurement: u8,
    pub content_changed: u8,
    pub slot_id: u8,
    pub measurement_record: SpdmMeasurementRecordStructure,
    pub nonce: SpdmNonceStruct,
    pub opaque: SpdmOpaqueStruct,
    pub signature: SpdmSignatureStruct,
}

impl SpdmCodec for SpdmMeasurementsResponsePayload {
    fn spdm_encode(&self, context: &mut common::SpdmContext, bytes: &mut Writer) {
        //When Param2 in the requested measurement operation is 0 , this
        //parameter shall return the total number of measurement indices on
        //the device. Otherwise, this field is reserved.
        if self.number_of_measurement == 1 {
            0_u8.encode(bytes); // param1
        } else {
            self.number_of_measurement.encode(bytes); // param1
        }
        if context.negotiate_info.spdm_version_sel == SpdmVersion::SpdmVersion12
            && context.config_info.runtime_content_change_support
        {
            (self.slot_id | self.content_changed).encode(bytes); // param2
        } else {
            self.slot_id.encode(bytes); // param 2
        }
        self.measurement_record.spdm_encode(context, bytes);
        self.nonce.encode(bytes);
        self.opaque.spdm_encode(context, bytes);
        if context.runtime_info.need_measurement_signature {
            self.signature.spdm_encode(context, bytes);
        }
    }

    fn spdm_read(
        context: &mut common::SpdmContext,
        r: &mut Reader,
    ) -> Option<SpdmMeasurementsResponsePayload> {
        let number_of_measurement = u8::read(r)?; // param1
        let param2 = u8::read(r)?; // param2
        let slot_id = param2 & MEASUREMENT_RESPONDER_PARAM2_SLOT_ID_MASK; // Bit [3:0]
        let content_changed = param2 & MEASUREMENT_RESPONDER_PARAM2_CONTENT_CHANGED_MASK; // Bit [5:4]
        let measurement_record = SpdmMeasurementRecordStructure::spdm_read(context, r)?;
        let nonce = SpdmNonceStruct::read(r)?;
        let opaque = SpdmOpaqueStruct::spdm_read(context, r)?;
        let signature = if context.runtime_info.need_measurement_signature {
            SpdmSignatureStruct::spdm_read(context, r)?
        } else {
            SpdmSignatureStruct::default()
        };
        Some(SpdmMeasurementsResponsePayload {
            number_of_measurement,
            content_changed,
            slot_id,
            measurement_record,
            nonce,
            opaque,
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::*;
    use crate::config::*;
    use crate::testlib::*;

    #[test]
    fn test_case0_spdm_spdm_measuremente_attributes() {
        let u8_slice = &mut [0u8; 4];
        let mut writer = Writer::init(u8_slice);
        let value = SpdmMeasurementeAttributes::SIGNATURE_REQUESTED;
        value.encode(&mut writer);

        let mut reader = Reader::init(u8_slice);
        assert_eq!(4, reader.left());
        assert_eq!(
            SpdmMeasurementeAttributes::read(&mut reader).unwrap(),
            SpdmMeasurementeAttributes::SIGNATURE_REQUESTED
        );
        assert_eq!(3, reader.left());
    }
    #[test]
    fn test_case0_spdm_get_measurements_request_payload() {
        let u8_slice = &mut [0u8; 48];
        let mut writer = Writer::init(u8_slice);
        let value = SpdmGetMeasurementsRequestPayload {
            measurement_attributes: SpdmMeasurementeAttributes::SIGNATURE_REQUESTED,
            measurement_operation: SpdmMeasurementOperation::SpdmMeasurementQueryTotalNumber,
            nonce: SpdmNonceStruct {
                data: [100u8; common::algo::SPDM_NONCE_SIZE],
            },
            slot_id: 0xaau8,
        };

        let pcidoe_transport_encap = &mut PciDoeTransportEncap {};
        let my_spdm_device_io = &mut MySpdmDeviceIo;
        let mut context = new_context(my_spdm_device_io, pcidoe_transport_encap);

        value.spdm_encode(&mut context, &mut writer);
        let mut reader = Reader::init(u8_slice);
        assert_eq!(48, reader.left());
        let get_measurements =
            SpdmGetMeasurementsRequestPayload::spdm_read(&mut context, &mut reader).unwrap();
        assert_eq!(
            get_measurements.measurement_attributes,
            SpdmMeasurementeAttributes::SIGNATURE_REQUESTED
        );
        assert_eq!(
            get_measurements.measurement_operation,
            SpdmMeasurementOperation::SpdmMeasurementQueryTotalNumber,
        );
        assert_eq!(get_measurements.slot_id, 0xaau8);
        for i in 0..32 {
            assert_eq!(get_measurements.nonce.data[i], 100u8);
        }
        assert_eq!(13, reader.left());
    }
    #[test]
    fn test_case1_spdm_get_measurements_request_payload() {
        let u8_slice = &mut [0u8; 48];
        let mut writer = Writer::init(u8_slice);
        let value = SpdmGetMeasurementsRequestPayload {
            measurement_attributes: SpdmMeasurementeAttributes::empty(),
            measurement_operation: SpdmMeasurementOperation::SpdmMeasurementQueryTotalNumber,
            nonce: SpdmNonceStruct {
                data: [100u8; common::algo::SPDM_NONCE_SIZE],
            },
            slot_id: 0xaau8,
        };

        let pcidoe_transport_encap = &mut PciDoeTransportEncap {};
        let my_spdm_device_io = &mut MySpdmDeviceIo;
        let mut context = new_context(my_spdm_device_io, pcidoe_transport_encap);

        value.spdm_encode(&mut context, &mut writer);
        let mut reader = Reader::init(u8_slice);
        assert_eq!(48, reader.left());
        let get_measurements =
            SpdmGetMeasurementsRequestPayload::spdm_read(&mut context, &mut reader).unwrap();
        assert_eq!(
            get_measurements.measurement_attributes,
            SpdmMeasurementeAttributes::empty()
        );
        assert_eq!(
            get_measurements.measurement_operation,
            SpdmMeasurementOperation::SpdmMeasurementQueryTotalNumber,
        );
        assert_eq!(get_measurements.slot_id, 0);
        for i in 0..32 {
            assert_eq!(get_measurements.nonce.data[i], 0);
        }
        assert_eq!(46, reader.left());
    }
    #[test]
    fn test_case0_spdm_measurements_response_payload() {
        let u8_slice = &mut [0u8; 1000];
        let mut writer = Writer::init(u8_slice);
        let value = SpdmMeasurementsResponsePayload {
            number_of_measurement: 100u8,
            slot_id: 7u8,
            content_changed: MEASUREMENT_RESPONDER_PARAM2_CONTENT_CHANGED_NOT_SUPPORTED_VALUE,
            measurement_record: SpdmMeasurementRecordStructure {
                number_of_blocks: 5,
                record: gen_array_clone(
                    common::algo::SpdmMeasurementBlockStructure {
                    index: 100u8,
                    measurement_specification: common::algo::SpdmMeasurementSpecification::DMTF,
                    measurement_size: 67u16,
                    measurement: common::algo::SpdmDmtfMeasurementStructure {
                        r#type: common::algo::SpdmDmtfMeasurementType::SpdmDmtfMeasurementRom,
                        representation:
                        common::algo::SpdmDmtfMeasurementRepresentation::SpdmDmtfMeasurementDigest,
                        value_size: 64u16,
                        value: [100u8; MAX_SPDM_MEASUREMENT_VALUE_LEN],
                    },
                },MAX_SPDM_MEASUREMENT_BLOCK_COUNT),
            },
            nonce: SpdmNonceStruct {
                data: [100u8; common::algo::SPDM_NONCE_SIZE],
            },
            opaque: SpdmOpaqueStruct {
                data_size: 64,
                data: [100u8; MAX_SPDM_OPAQUE_SIZE],
            },
            signature: SpdmSignatureStruct {
                data_size: 512,
                data: [100u8; common::algo::SPDM_MAX_ASYM_KEY_SIZE],
            },
        };

        let pcidoe_transport_encap = &mut PciDoeTransportEncap {};
        let my_spdm_device_io = &mut MySpdmDeviceIo;
        let mut context = new_context(my_spdm_device_io, pcidoe_transport_encap);
        context.negotiate_info.base_asym_sel = common::algo::SpdmBaseAsymAlgo::TPM_ALG_RSASSA_4096;
        context.negotiate_info.base_hash_sel = common::algo::SpdmBaseHashAlgo::TPM_ALG_SHA_512;
        context.runtime_info.need_measurement_signature = true;
        value.spdm_encode(&mut context, &mut writer);
        let mut reader = Reader::init(u8_slice);

        assert_eq!(1000, reader.left());
        let mut measurements_response =
            SpdmMeasurementsResponsePayload::spdm_read(&mut context, &mut reader).unwrap();
        assert_eq!(measurements_response.number_of_measurement, 100);
        assert_eq!(measurements_response.slot_id, 7);
        assert_eq!(
            measurements_response.content_changed,
            MEASUREMENT_RESPONDER_PARAM2_CONTENT_CHANGED_NOT_SUPPORTED_VALUE
        );

        assert_eq!(measurements_response.measurement_record.number_of_blocks, 5);
        for i in 0..5 {
            assert_eq!(
                measurements_response.measurement_record.record[i].index,
                100
            );
            assert_eq!(
                measurements_response.measurement_record.record[i].measurement_specification,
                common::algo::SpdmMeasurementSpecification::DMTF
            );
            assert_eq!(
                measurements_response.measurement_record.record[i].measurement_size,
                67
            );
            assert_eq!(
                measurements_response.measurement_record.record[i]
                    .measurement
                    .r#type,
                common::algo::SpdmDmtfMeasurementType::SpdmDmtfMeasurementRom
            );
            assert_eq!(
                measurements_response.measurement_record.record[i]
                    .measurement
                    .representation,
                common::algo::SpdmDmtfMeasurementRepresentation::SpdmDmtfMeasurementDigest
            );
            assert_eq!(
                measurements_response.measurement_record.record[i]
                    .measurement
                    .value_size,
                64
            );
            for j in 0..64 {
                assert_eq!(
                    measurements_response.measurement_record.record[i]
                        .measurement
                        .value[j],
                    100
                );
            }
        }
        for i in 0..32 {
            assert_eq!(measurements_response.nonce.data[i], 100);
        }

        assert_eq!(measurements_response.opaque.data_size, 64);
        for i in 0..64 {
            assert_eq!(measurements_response.opaque.data[i], 100);
        }

        assert_eq!(measurements_response.signature.data_size, 512);
        for i in 0..512 {
            assert_eq!(measurements_response.signature.data[i], 100);
        }
        assert_eq!(29, reader.left());

        let u8_slice = &mut [0u8; 1000];
        let mut writer = Writer::init(u8_slice);

        context.runtime_info.need_measurement_signature = false;
        value.spdm_encode(&mut context, &mut writer);
        let mut reader = Reader::init(u8_slice);
        assert_eq!(1000, reader.left());
        measurements_response =
            SpdmMeasurementsResponsePayload::spdm_read(&mut context, &mut reader).unwrap();

        assert_eq!(measurements_response.signature.data_size, 0);

        for i in 0..32 {
            assert_eq!(measurements_response.nonce.data[i], 100);
        }
        for i in 0..512 {
            assert_eq!(measurements_response.signature.data[i], 0);
        }
        assert_eq!(541, reader.left());
    }
}
