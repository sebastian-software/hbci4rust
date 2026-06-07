use crate::error::HbciResult;
use crate::passport::UserSig;
use crate::protocol::HbciMessage;

pub fn apply_pintan_user_sig_to_sig_tail(
    message: &mut HbciMessage,
    sig_tail_path: &str,
    signature: &[u8],
) -> HbciResult<()> {
    let user_sig = UserSig::decode(Some(signature))?;

    message.set_value(&format!("{sig_tail_path}.UserSig.pin"), user_sig.pin())?;
    if !user_sig.tan().is_empty() {
        message.set_value(&format!("{sig_tail_path}.UserSig.tan"), user_sig.tan())?;
    }

    Ok(())
}
