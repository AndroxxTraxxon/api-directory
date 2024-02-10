use async_trait::async_trait;

struct UserQueryParams<'a> {
    pub username: &'a String,
    pub password_hash: &'a String
}

// #[async_trait]
// pub trait UserServiceRepository {
//     async fn authenticate_user(username: &String, password: &String) -> Result<>{
//         todo!()
//     }
// }