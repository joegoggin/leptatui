/// Minimal renderable node tree for hand-written terminal UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node {
    /// Bordered container around a child node.
    Block {
        /// Child node rendered inside the block's inner area.
        child: Box<Node>,
    },
    /// Plain text content.
    Text(String),
    /// Horizontally arranged children.
    Row(Vec<Node>),
    /// Vertically arranged children.
    Column(Vec<Node>),
    /// Basic bordered button label.
    Button(String),
}

impl From<String> for Node {
    /// Converts owned text into a text node.
    ///
    /// # Arguments
    ///
    /// * `value` — Text content to render.
    ///
    /// # Returns
    ///
    /// A [`Node::Text`] containing `value`.
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Node {
    /// Converts borrowed text into a text node.
    ///
    /// # Arguments
    ///
    /// * `value` — Text content to copy into the node.
    ///
    /// # Returns
    ///
    /// A [`Node::Text`] containing `value`.
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}
