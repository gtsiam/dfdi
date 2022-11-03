use dfdi::{
    util::{Cached, CachedRef},
    Context, Service,
};

#[derive(Debug, Clone, Service)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Debug, Service)]
#[service(error = UserError)]
struct User<'cx> {
    #[allow(unused)]
    username: &'cx str,
}

#[derive(Debug, thiserror::Error)]
enum UserError {
    #[error("invalid authorization token")]
    InvalidAuth,
}

fn main() {
    let mut cx = Context::new();

    let credentials = Credentials {
        username: "admin".to_string(),
        password: "admin".to_string(),
    };

    cx.bind_with::<&Credentials>(CachedRef(credentials));

    cx.bind_with::<&User>(Cached::new_fn(|cx| {
        let token = cx.resolve::<&Credentials>();
        match (&*token.username, &*token.password) {
            (username @ "admin", "admin") => Ok(User { username }),
            _ => Err(UserError::InvalidAuth),
        }
    }));

    println!("AuthToken: {:?}", cx.resolve::<&Credentials>());
    println!("User: {:?}", cx.resolve::<&User>().as_ref().unwrap());
}
