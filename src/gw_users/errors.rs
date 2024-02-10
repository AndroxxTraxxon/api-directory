use derive_more::Display;

#[derive(Debug, Display)]
pub enum UserError {
    UserNotFound,
    UserRegistrationFailure,
    AuthenticationFailure,

}