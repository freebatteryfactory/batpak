use super::ReactorError;

#[test]
fn display_renders_each_variant_text() {
    // Pins the Display impl: stubbing it to `Ok(Default::default())` would
    // emit an empty string for every variant and erase the diagnostic.
    let user: ReactorError<std::io::Error> =
        ReactorError::User(std::io::Error::other("handler blew up"));
    assert_eq!(user.to_string(), "reactor user error: handler blew up");

    let exhausted: ReactorError<std::io::Error> = ReactorError::RestartBudgetExhausted;
    assert_eq!(exhausted.to_string(), "reactor restart budget exhausted");
}
