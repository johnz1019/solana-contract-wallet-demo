use anchor_lang::prelude::*;

/// Custom error codes
#[error_code]
pub enum ErrorCode {
    #[msg("Signature verification failed1.")]
    SigVerificationFailed1,
    #[msg("Signature verification failed2.")]
    SigVerificationFailed2,
    #[msg("Signature verification failed3.")]
    SigVerificationFailed3,
}
