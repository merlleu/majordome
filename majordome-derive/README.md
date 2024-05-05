# Majordome Errors Enums
```rs
#[derive(MajordomeError)]
#[err(prefix = "errors.gg.wls.")]
pub enum AuthError {
    #[err(code="invalid_token", msg="Invalid token", status=401)]
    InvalidToken,

    #[err(code="unknown_event", msg="Unknown event {id}", status=404)]
    UnknownEvent {id: String},

    #[err(code="not_enough_players", msg="Not enough players (required: {required}, actual: {actual})", status=400)]
    NotEnoughPlayers{required: u32, actual: u32},
}
```