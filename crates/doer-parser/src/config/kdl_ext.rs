use kdl::{KdlDocument, KdlEntry, KdlNode};

pub trait KdlNodeExt {
    fn first_entry(&self) -> Option<&KdlEntry>;
    fn first_string(&self) -> Option<&str>;
}

impl KdlNodeExt for KdlNode {
    fn first_entry(&self) -> Option<&KdlEntry> {
        self.entries().first()
    }

    fn first_string(&self) -> Option<&str> {
        self.first_entry().and_then(|e| e.string_value())
    }
}

pub trait KdlDocumentExt {
    fn nodes_by_name(&self, name: &str) -> Vec<&KdlNode>;
}

impl KdlDocumentExt for KdlDocument {
    fn nodes_by_name(&self, name: &str) -> Vec<&KdlNode> {
        self.nodes().iter().filter(|n| n.name().value() == name).collect()
    }
}

pub trait KdlEntryExt {
    fn key(&self) -> Option<&str>;
    fn string_value(&self) -> Option<&str>;
}

impl KdlEntryExt for KdlEntry {
    fn key(&self) -> Option<&str> {
        self.name().map(|n| n.value())
    }

    fn string_value(&self) -> Option<&str> {
        self.value().as_string()
    }
}
