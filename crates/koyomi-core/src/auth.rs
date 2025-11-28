pub fn login() {
    println!("User logged in!");
}

pub fn logout() {
    println!("User logged out!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login() {
        login();
    }

    #[test]
    fn test_logout() {
        logout();
    }
}
