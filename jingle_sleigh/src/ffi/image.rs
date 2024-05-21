#[cxx::bridge]
pub(crate) mod bridge {
    #[derive(Debug, Clone)]
    pub struct Perms {
        pub(crate) read: bool,
        pub(crate) write: bool,
        pub(crate) exec: bool,
    }

    #[derive(Debug, Clone)]
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
