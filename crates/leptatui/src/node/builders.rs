use super::model::Node;

/// Creates a bordered block around a child node.
///
/// # Arguments
///
/// * `child` — Node-compatible value rendered inside the block.
///
/// # Returns
///
/// A [`Node::Block`] containing the provided child.
pub fn block(child: impl Into<Node>) -> Node {
    Node::Block {
        child: Box::new(child.into()),
    }
}

/// Creates a text node.
///
/// # Arguments
///
/// * `content` — Text content to render.
///
/// # Returns
///
/// A [`Node::Text`] containing the provided content.
pub fn text(content: impl Into<String>) -> Node {
    Node::Text(content.into())
}

/// Creates a horizontal row.
///
/// # Arguments
///
/// * `children` — Child nodes to divide across the row.
///
/// # Returns
///
/// A [`Node::Row`] containing the provided children.
pub fn row(children: impl IntoIterator<Item = Node>) -> Node {
    Node::Row(children.into_iter().collect())
}

/// Creates a vertical column.
///
/// # Arguments
///
/// * `children` — Child nodes to divide down the column.
///
/// # Returns
///
/// A [`Node::Column`] containing the provided children.
pub fn column(children: impl IntoIterator<Item = Node>) -> Node {
    Node::Column(children.into_iter().collect())
}

/// Creates a basic button.
///
/// # Arguments
///
/// * `label` — Button text to center inside a bordered area.
///
/// # Returns
///
/// A [`Node::Button`] containing the provided label.
pub fn button(label: impl Into<String>) -> Node {
    Node::Button(label.into())
}
