use validator::ValidationError;

pub fn _validator_password(password:&str) -> Result<(), ValidationError> {

    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if has_upper && has_lower && has_digit && has_special && password.len() >= 8{
        Ok(())
    }else {
        let mut err = ValidationError::new("weak_password");
        err.message = Some("Пароль має містити великі та малі літери, цифри та спецсимвол".into());
        Err(err)
    }
}