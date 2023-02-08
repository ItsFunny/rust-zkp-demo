// listen l1 event and sync the data
pub struct Syncer {
    pub l1Address: String,
}

impl Syncer {
    pub fn new(l1Addr: String) -> Syncer {
        Syncer { l1Address: l1Addr }
    }
}