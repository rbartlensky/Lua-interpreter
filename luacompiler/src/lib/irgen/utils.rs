use lrpar::Node::{self, *};

/// Get the children of the given node if its ridx is equal to <ridx>.
/// # Panic
/// This function panics when the node is a `Term` or its ridx is not the same as the
/// one given.
pub fn get_nodes(node: &Node<u8>, ridx: u8) -> &Vec<Node<u8>> {
    match *node {
        Nonterm { ridx: _, ref nodes } => nodes,
        _ => panic!(
            "Expected a Nonterm node with id {}, but got {:#?}",
            ridx, node
        ),
    }
}

/// Check if <node> is a term, and has the token id equal to <id>.
pub fn is_term(node: &Node<u8>, id: u8) -> bool {
    match node {
        Term { lexeme } if lexeme.tok_id() == id => true,
        _ => false,
    }
}

/// Find the first Node::Term with the given id.
pub fn find_term(start: &Node<u8>, id: u8) -> Option<&Node<u8>> {
    let mut pt_nodes: Vec<&Node<u8>> = vec![start];
    while !pt_nodes.is_empty() {
        let node = pt_nodes.pop().unwrap();
        match node {
            Nonterm { ridx: _, ref nodes } => {
                for ref node in nodes {
                    pt_nodes.push(node);
                }
            }
            Term { lexeme } => {
                if lexeme.tok_id() == id {
                    return Some(node);
                }
            }
        }
    }
    None
}
