use std::collections::HashSet;

pub struct Tags(HashSet<String>);

impl Tags {
  pub fn new() -> Tags {
    Tags(HashSet::new())
  }

  pub fn add_tag(&mut self, tag: String) {
    self.0.insert(tag);
  }

  pub fn remove_tag(&mut self, tag: &String) {
    self.0.remove(tag);
  }

  pub fn has_tag(&self, tag: &String) -> bool {
    self.0.contains(tag)
  }

  pub fn toggle_tag(&mut self, tag: String) {
    if self.has_tag(&tag) {
      self.remove_tag(&tag);
    } else {
      self.add_tag(tag);
    }
  }
}
