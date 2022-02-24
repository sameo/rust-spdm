// Copyright (c) 2022 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use codec::{Codec, Reader};

use crate::common::error::{spdm_err, spdm_result_err, SpdmResult};
use crate::common::session::SpdmSessionState;
use crate::message::*;
use crate::requester::RequesterContext;
use crate::time::sleep;

impl<'a> RequesterContext<'a> {
    fn spdm_handle_response_not_ready(
        &mut self,
        _session_id: u32,
        response: &[u8],
        original_request_code: SpdmRequestResponseCode,
        expected_response_code: SpdmRequestResponseCode,
    ) -> SpdmResult<ReceivedMessage> {
        if response.len()
            != core::mem::size_of::<SpdmMessageHeader>()
                + core::mem::size_of::<SpdmMessageGeneralPayload>()
                + core::mem::size_of::<SpdmErrorResponseNotReadyExtData>()
        {
            return spdm_result_err!(EDEV);
        } else {
            let extoff = core::mem::size_of::<SpdmMessageHeader>()
                + core::mem::size_of::<SpdmMessageGeneralPayload>();
            let mut extend_error_data_reader = Reader::init(&response[extoff..]);
            let extend_error_data =
                SpdmErrorResponseNotReadyExtData::read(&mut extend_error_data_reader).unwrap();

            if extend_error_data.request_code != original_request_code.get_u8() {
                return spdm_result_err!(EDEV);
            }

            sleep(2 << extend_error_data.rdt_exponent);

            self.spdm_requester_respond_if_ready(expected_response_code, extend_error_data)
        }
    }

    fn spdm_handle_simple_error_response(
        &mut self,
        session_id: u32,
        error_code: u8,
    ) -> SpdmResult<ReceivedMessage> {
        /* NOT_READY is treated as error here.
         * Use spdm_handle_error_response_main to handle NOT_READY message in long latency command.*/
        if error_code == SpdmErrorCode::SpdmErrorResponseNotReady.get_u8() {
            return spdm_result_err!(EDEV);
        } else if error_code == SpdmErrorCode::SpdmErrorBusy.get_u8() {
            return spdm_result_err!(EBUSY);
        } else if error_code == SpdmErrorCode::SpdmErrorRequestResynch.get_u8() {
            let mut session = self.common.get_session_via_id(session_id).unwrap().clone();
            session.set_session_state(SpdmSessionState::SpdmSessionNotStarted);
            return spdm_result_err!(EDEV);
        } else {
            return spdm_result_err!(EDEV);
        }
    }

    pub fn spdm_handle_error_response_main(
        &mut self,
        session_id: u32,
        response: &[u8],
        original_request_code: SpdmRequestResponseCode,
        expected_response_code: SpdmRequestResponseCode,
    ) -> SpdmResult<ReceivedMessage> {
        let mut spdm_message_header_reader = Reader::init(response);
        let spdm_message_header = SpdmMessageHeader::read(&mut spdm_message_header_reader).unwrap();
        let header_size = spdm_message_header_reader.used();
        assert_eq!(
            spdm_message_header.version,
            self.common.negotiate_info.spdm_version_sel
        );
        assert_eq!(
            spdm_message_header.request_response_code,
            SpdmRequestResponseCode::SpdmResponseError
        );

        let mut spdm_message_payload_reader = Reader::init(&response[header_size..]);
        let spdm_message_general_payload =
            SpdmMessageGeneralPayload::read(&mut spdm_message_payload_reader).unwrap();

        if spdm_message_general_payload.param1 == SpdmErrorCode::SpdmErrorDecryptError.get_u8() {
            let session = self.common.get_session_via_id(session_id).unwrap();
            let _ = session.teardown(session_id);

            spdm_result_err!(ESEC)
        } else if spdm_message_general_payload.param1
            == SpdmErrorCode::SpdmErrorResponseNotReady.get_u8()
        {
            self.spdm_handle_response_not_ready(
                session_id,
                response,
                original_request_code,
                expected_response_code,
            )
        } else {
            self.spdm_handle_simple_error_response(session_id, spdm_message_general_payload.param1)
        }
    }
}
