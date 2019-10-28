use super::Error;
use log::{debug, error};
use std::collections::HashMap;

#[derive(PartialEq, Debug)]
enum XmlTagType {
    Opening,
    SelfClosing,
    Closing,
}
#[derive(Debug)]
pub struct XmlTag {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub value: Option<String>,
    id: usize,
    parent: Option<usize>,
    children: Vec<usize>,
    tag_type: XmlTagType,
}
impl XmlTag {
    fn parse<T>(iter: T, id: usize) -> Result<XmlTag, Error>
    where
        T: Iterator<Item = String>,
    {
        let tokens: Vec<String> = iter.take_while(|v| !v.ends_with('>')).collect();
        debug!("Tokens: {:#?}", tokens);
        let tag_type = if tokens[1].chars().nth(0).unwrap() == '/' {
            XmlTagType::Closing
        } else if tokens.last().unwrap().chars().nth(0).unwrap() == '/' {
            XmlTagType::SelfClosing
        } else {
            XmlTagType::Opening
        };
        debug!("Tag type type: {:?}", tag_type);
        let mut xml_tag = XmlTag {
            name: tokens[1].to_string(),
            attributes: HashMap::new(),
            value: None,
            id,
            parent: None,
            children: vec![],
            tag_type,
        };
        if tokens.len() > 2 {
            xml_tag.attributes = HashMap::new();
            for token in &tokens[2..] {
                let mut token: std::str::Split<_> = token.split('=');
                xml_tag.attributes.insert(
                    token.next().unwrap().to_string(),
                    token
                        .next()
                        .unwrap_or(&"")
                        .trim_end_matches(|v| v != '\'' && v != '\"')
                        .to_string(),
                );
            }
        }
        Ok(xml_tag)
    }
    pub fn get_id(&self) -> usize {
        self.id
    }
}
impl std::fmt::Display for XmlTag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let text = match self.tag_type {
            XmlTagType::SelfClosing => format!(
                "<{name} {attributes}/>",
                name = self.name,
                attributes = self
                    .attributes
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<String>()
            ),
            XmlTagType::Opening => format!(
                "<{name} {attributes}>{value}",
                name = self.name,
                attributes = self
                    .attributes
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<String>(),
                value = match self.value {
                    Some(ref v) => &v,
                    None => "",
                }
            ),
            XmlTagType::Closing => format!("</{name}>", name = self.name),
        };

        write!(f, "{}", text)
    }
}
pub struct XmlTree {
    nodes: Vec<XmlTag>,
}
impl XmlTree {
    /* #region Parsing */
    pub fn parse(text: String) -> Result<Self, Error> {
        let tokens: _ = text
            .replace("<", "\n< ") // These 3 add whitespaces around the start and end of the tags so they can be easily split with the next function
            .replace(">", " >\n") // like this: <rdf::RDF> --> \n<rdf:RDF\s>\n
            .replace("/ >", " />") // transform /\s> into \s/>
            .split_ascii_whitespace()
            .skip_while(|v| *v != "<") // Skip untl the begining of the file
            .map(|v: &str| v.to_string()) // Transform everything into Strings
            .collect::<Vec<String>>();
        let mut tree = XmlTree { nodes: vec![] };
        let mut parent_stack: Vec<usize> = vec![];
        let mut tokens_iter: std::iter::Peekable<_> = tokens.into_iter().peekable();
        while let Some(value_peeked) = tokens_iter.peek() {
            if value_peeked.starts_with('<') {
                let tag = XmlTag::parse(&mut tokens_iter, tree.get_next_id())?;
                match tag.tag_type {
                    XmlTagType::Opening => {
                        let inserted_tag_id = tree.push(tag);
                        if !parent_stack.is_empty() {
                            let parent = parent_stack.last().unwrap();
                            tree.link(*parent, inserted_tag_id);
                        }
                        parent_stack.push(inserted_tag_id);
                    }
                    XmlTagType::SelfClosing => {
                        let inserted_tag_id = tree.push(tag);
                        if !parent_stack.is_empty() {
                            let parent = parent_stack.last().unwrap();
                            tree.link(*parent, inserted_tag_id);
                        }
                    }
                    XmlTagType::Closing => {
                        if parent_stack.pop().is_none() {
                            error!("Closing tag without opening");
                            return Err(Error::Parser);
                        }
                    }
                }
            } else {
                match tree.nodes.last_mut() {
                    Some(node) => node.value = Some(tokens_iter.next().unwrap().to_string()),
                    None => return Err(Error::Parser),
                }
            }
        }
        Ok(tree)
    }
    fn link(&mut self, parent: usize, child: usize) {
        self[child].parent = Some(parent);
        self[parent].children.push(child);
        self[parent].children.sort();
        self[parent].children.dedup();
    }
    fn push(&mut self, tag: XmlTag) -> usize {
        self.nodes.push(tag);
        self.nodes.len() - 1
    }
    fn get_next_id(&self) -> usize {
        self.nodes.len()
    }
    /* #endregion */
    pub fn find_elements<F>(&self, find_function: F) -> Vec<usize>
    where
        F: Fn(&XmlTag) -> bool,
    {
        let mut finds: Vec<usize> = vec![];
        self.traverse_map(
            0,
            |e, mut v: Option<_>| {
                if find_function(e) {
                    let finds = v.unwrap();
                    finds.push(e.get_id());
                    v = Some(finds);
                }
                v
            },
            Some(&mut finds),
        );
        finds
    }
    pub fn traverse_map<F, V>(&self, start: usize, mut f: F, v: V) -> V
    where
        F: FnMut(&XmlTag, V) -> V + Copy,
    {
        let mut last_val: V = f(&self[start], v);
        for child in &self[start].children {
            last_val = self.traverse_map(*child, f, last_val);
        }
        last_val
    }
}
impl std::fmt::Display for XmlTree {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let printable = self
            .traverse_map(
                0,
                |tag, val| Some(format!("{}{}", val.unwrap(), tag)),
                Some(String::from("")),
            )
            .unwrap();
        write!(f, "{}", printable)
    }
}
impl std::ops::Index<usize> for XmlTree {
    type Output = XmlTag;
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.nodes.len() {
            panic!("Index out of bounds");
        }
        self.nodes.get(index).unwrap()
    }
}
impl std::ops::IndexMut<usize> for XmlTree {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.nodes.len() {
            panic!("Index out of bounds");
        }
        self.nodes.get_mut(index).unwrap()
    }
}
