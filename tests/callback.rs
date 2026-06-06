use hbci4rust::{CallbackDataType, CallbackReason};

#[test]
fn callback_data_type_codes_match_hbci4java_constants() {
    assert_eq!(CallbackDataType::None.original_code(), 0);
    assert_eq!(CallbackDataType::Secret.original_code(), 1);
    assert_eq!(CallbackDataType::Text.original_code(), 2);
    assert_eq!(CallbackDataType::Boolean.original_code(), 3);

    assert_eq!(CallbackDataType::TYPE_NONE, 0);
    assert_eq!(CallbackDataType::TYPE_SECRET, 1);
    assert_eq!(CallbackDataType::TYPE_TEXT, 2);
    assert_eq!(CallbackDataType::TYPE_BOOLEAN, 3);
}

#[test]
fn callback_data_type_decodes_original_codes() {
    assert_eq!(
        CallbackDataType::from_original_code(CallbackDataType::TYPE_NONE),
        CallbackDataType::None
    );
    assert_eq!(
        CallbackDataType::from_original_code(CallbackDataType::TYPE_SECRET),
        CallbackDataType::Secret
    );
    assert_eq!(
        CallbackDataType::from_original_code(CallbackDataType::TYPE_TEXT),
        CallbackDataType::Text
    );
    assert_eq!(
        CallbackDataType::from_original_code(CallbackDataType::TYPE_BOOLEAN),
        CallbackDataType::Boolean
    );
    assert_eq!(
        CallbackDataType::from_original_code(99),
        CallbackDataType::Unknown(99)
    );
}

#[test]
fn callback_select_uses_original_text_type_boundary() {
    assert_eq!(
        CallbackDataType::Select.original_code(),
        CallbackDataType::TYPE_TEXT
    );
    assert_eq!(
        CallbackDataType::from_original_code(CallbackDataType::TYPE_TEXT),
        CallbackDataType::Text
    );
}

#[test]
fn callback_reason_codes_match_hbci4java_constants_for_ported_variants() {
    assert_eq!(CallbackReason::NeedCountry.original_code(), 7);
    assert_eq!(CallbackReason::NeedBlz.original_code(), 8);
    assert_eq!(CallbackReason::NeedHost.original_code(), 9);
    assert_eq!(CallbackReason::NeedPort.original_code(), 10);
    assert_eq!(CallbackReason::NeedUserId.original_code(), 11);
    assert_eq!(CallbackReason::HaveInstMsg.original_code(), 14);
    assert_eq!(CallbackReason::NeedPtPin.original_code(), 16);
    assert_eq!(CallbackReason::NeedPtTan.original_code(), 17);
    assert_eq!(CallbackReason::NeedCustomerId.original_code(), 18);
    assert_eq!(CallbackReason::HaveCrcError.original_code(), 19);
    assert_eq!(CallbackReason::HaveError.original_code(), 20);
    assert_eq!(CallbackReason::NeedConnection.original_code(), 24);
    assert_eq!(CallbackReason::CloseConnection.original_code(), 25);
    assert_eq!(CallbackReason::NeedFilter.original_code(), 26);
    assert_eq!(CallbackReason::NeedPtSecMech.original_code(), 27);
    assert_eq!(CallbackReason::HaveIbanError.original_code(), 30);
    assert_eq!(CallbackReason::NeedPtTanMedia.original_code(), 32);

    assert_eq!(CallbackReason::NEED_COUNTRY, 7);
    assert_eq!(CallbackReason::HAVE_INST_MSG, 14);
    assert_eq!(CallbackReason::HAVE_CRC_ERROR, 19);
    assert_eq!(CallbackReason::HAVE_IBAN_ERROR, 30);
    assert_eq!(CallbackReason::NEED_PT_TANMEDIA, 32);
}

#[test]
fn callback_reason_decodes_ported_original_codes() {
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::NEED_CONNECTION),
        CallbackReason::NeedConnection
    );
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::CLOSE_CONNECTION),
        CallbackReason::CloseConnection
    );
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::HAVE_INST_MSG),
        CallbackReason::HaveInstMsg
    );
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::HAVE_CRC_ERROR),
        CallbackReason::HaveCrcError
    );
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::HAVE_IBAN_ERROR),
        CallbackReason::HaveIbanError
    );
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::NEED_PT_PIN),
        CallbackReason::NeedPtPin
    );
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::NEED_PT_TAN),
        CallbackReason::NeedPtTan
    );
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::NEED_PT_SECMECH),
        CallbackReason::NeedPtSecMech
    );
    assert_eq!(
        CallbackReason::from_original_code(CallbackReason::NEED_PT_TANMEDIA),
        CallbackReason::NeedPtTanMedia
    );
    assert_eq!(
        CallbackReason::from_original_code(2),
        CallbackReason::Unknown(2)
    );
    assert_eq!(CallbackReason::Unknown(99).original_code(), 99);
}
