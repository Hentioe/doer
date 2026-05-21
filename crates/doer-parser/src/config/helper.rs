use crate::prelude::*;
use kdl::KdlNode;

pub fn ensure_entries_count(node: &KdlNode, expected: usize, label: &str) -> Result<()> {
    let actual = node.entries().len();
    ensure!(
        actual == expected,
        "{}: expected {} entries, got {}",
        label,
        expected,
        actual
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kdl::KdlDocument;

    fn first_node(doc: &KdlDocument) -> &KdlNode {
        doc.nodes().first().unwrap()
    }

    fn parse_doc(kdl: &str) -> KdlDocument {
        kdl.parse().unwrap()
    }

    #[test]
    fn entries_count_valid() {
        let doc = parse_doc(r#"arg "value""#);
        let node = first_node(&doc);
        assert!(ensure_entries_count(node, 1, "arg").is_ok());
    }

    #[test]
    fn entries_count_too_few() {
        let doc = parse_doc("arg");
        let node = first_node(&doc);
        let err = ensure_entries_count(node, 1, "arg").unwrap_err();
        assert!(format!("{:#}", err).contains("expected 1 entries, got 0"));
    }

    #[test]
    fn entries_count_too_many() {
        let doc = parse_doc(r#"arg "a" "b""#);
        let node = first_node(&doc);
        let err = ensure_entries_count(node, 1, "arg").unwrap_err();
        assert!(format!("{:#}", err).contains("expected 1 entries, got 2"));
    }
}
