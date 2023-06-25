pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

pub struct OAuth {
    pub grant_type: String,
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
}

pub enum Auth {
    Basic(BasicAuth),
    OAuth(OAuth),
}
