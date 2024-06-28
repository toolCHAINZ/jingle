use std::fmt::{Debug, Formatter};

#[cxx::bridge]
pub(crate) mod bridge {
    #[derive(Debug, Clone)]
    pub struct Perms {
        pub(crate) read: bool,
        pub(crate) write: bool,
        pub(crate) exec: bool,
    }

    #[derive(Clone)]
    pub struct ImageSection {
        pub(crate) data: Vec<u8>,
        pub(crate) base_address: usize,
        pub(crate) perms: Perms,
    }

    #[derive(Debug, Clone)]
    pub struct Image {
        pub sections: Vec<ImageSection>,
    }
}

impl Debug for bridge::ImageSection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("ImageSection");
        d.field("base_address", &self.base_address)
            .field("perms", &self.perms);
        if self.data.len() > 16 {
            d.field("data", &format!("[ < {} bytes > ]", self.data.len()));
        } else {
            d.field("data", &self.data);
        }
        d.finish()
    }
}
