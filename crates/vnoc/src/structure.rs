//! The structure of the scope held in memory: folders, files, classes, functions. Skeleton: rebuilt from `vnoc/outline` on each
//! request; M1 puts it behind the store trait (SQLite).

use serde::{Deserialize, Serialize};
use vnoc_domain::{LocationKey, RepoPath, occurrences};
use vnoc_driver_protocol::{Def, DefKind, Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Folder,
    File,
    Class,
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// unique in the tree: the path for folders and files, the location key (`path::qualname#n`) for defs
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub path: String,
    pub qualname: Option<String>,
    pub range: Option<Range>,
    pub syntax_error: bool,
    pub children: Vec<Node>,
}

pub struct FileDefs {
    pub path: String,
    pub defs: Vec<Def>,
}

impl Node {
    fn folder(id: String, name: String, path: String) -> Node {
        Node { id, kind: Kind::Folder, name, path, qualname: None, range: None, syntax_error: false, children: vec![] }
    }

    /// How many nodes of each kind are under **and including** this one: [folders, files, classes, functions].
    /// (For a whole tree that counts the root folder too; the server reports folders *under* the root.)
    pub fn counts(&self) -> [usize; 4] {
        let mut c = [0usize; 4];
        c[self.kind as usize] += 1;
        for child in &self.children {
            let k = child.counts();
            for i in 0..4 {
                c[i] += k[i];
            }
        }
        c
    }

    /// folders first, then files, then by name; defs keep their source order
    fn sort(&mut self) {
        if matches!(self.kind, Kind::Folder) {
            self.children.sort_by(|a, b| (!matches!(a.kind, Kind::Folder), &a.name).cmp(&(!matches!(b.kind, Kind::Folder), &b.name)));
        }
        self.children.iter_mut().for_each(Node::sort);
    }
}

/// Builds the tree. A file whose path is not a valid repo path is left out (it cannot get a location key).
pub fn build(root_name: &str, files: Vec<FileDefs>) -> Node {
    let mut root = Node::folder("/".to_string(), root_name.to_string(), String::new());
    for file in files {
        let Ok(repo_path) = RepoPath::new(file.path.clone()) else { continue };
        let folder = file.path.rsplit_once('/').map_or("", |(dir, _)| dir);
        let name = file.path.rsplit('/').next().unwrap_or(&file.path).to_string();
        let parent = descend(&mut root, folder);
        parent.children.push(file_node(&repo_path, name, &file.defs));
    }
    root.sort();
    root
}

/// The folder node for `dir` (`"a/b"`), created on the way.
fn descend<'a>(root: &'a mut Node, dir: &str) -> &'a mut Node {
    let mut cur = root;
    let mut so_far = String::new();
    for part in dir.split('/').filter(|p| !p.is_empty()) {
        if !so_far.is_empty() {
            so_far.push('/');
        }
        so_far.push_str(part);
        let at = match cur.children.iter().position(|c| matches!(c.kind, Kind::Folder) && c.name == part) {
            Some(i) => i,
            None => {
                cur.children.push(Node::folder(so_far.clone(), part.to_string(), so_far.clone()));
                cur.children.len() - 1
            }
        };
        cur = &mut cur.children[at];
    }
    cur
}

fn file_node(path: &RepoPath, name: String, defs: &[Def]) -> Node {
    let mut file = Node { id: path.to_string(), kind: Kind::File, name, path: path.to_string(), qualname: None, range: None, syntax_error: defs.iter().any(|d| d.syntax_error), children: vec![] };
    // Defs come flat, in source order, parents before children. A def's parent is the NEAREST preceding def with the parent's
    // qualname still open, which stays right when a name is defined twice (two `top`s): pop until the top of the stack is the parent.
    let occ = occurrences(defs.iter().map(|d| d.qualname.as_str()));
    let mut open: Vec<(String, Vec<usize>)> = Vec::new(); // (qualname, index path from `file` to that node)
    for (def, occurrence) in defs.iter().zip(occ) {
        while open.last().is_some_and(|(q, _)| *q != def.parent) && !def.parent.is_empty() {
            open.pop();
        }
        if def.parent.is_empty() {
            open.clear();
        }
        let key = LocationKey::new(path.clone(), def.qualname.clone(), occurrence).expect("occurrences are 1-based").to_string();
        let node = Node {
            id: key,
            kind: if def.kind == DefKind::Class { Kind::Class } else { Kind::Function },
            name: def.name.clone(),
            path: path.to_string(),
            qualname: Some(def.qualname.clone()),
            range: Some(def.range),
            syntax_error: def.syntax_error,
            children: vec![],
        };
        let at = open.last().map(|(_, p)| p.clone()).unwrap_or_default();
        let mut target = &mut file;
        for i in &at {
            target = &mut target.children[*i];
        }
        target.children.push(node);
        let mut here = at;
        here.push(target.children.len() - 1);
        open.push((def.qualname.clone(), here));
    }
    file
}

#[cfg(test)]
mod tests {
    use super::*;
    use vnoc_driver_protocol::Position;

    fn def(qualname: &str, parent: &str, kind: DefKind) -> Def {
        let p = Position { line: 0, character: 0 };
        Def { qualname: qualname.into(), name: qualname.rsplit('.').next().unwrap().into(), kind, parent: parent.into(), parent_kind: None, range: Range { start: p, end: p }, params: vec![], id: None, syntax_error: false }
    }
    fn names(n: &Node) -> Vec<String> {
        n.children.iter().map(|c| c.name.clone()).collect()
    }

    #[test]
    fn folders_files_and_defs_nest_and_sort() {
        let tree = build("repo", vec![
            FileDefs { path: "b.py".into(), defs: vec![def("f", "", DefKind::Function)] },
            FileDefs { path: "pkg/sub/m.py".into(), defs: vec![] },
            FileDefs { path: "pkg/a.py".into(), defs: vec![def("C", "", DefKind::Class), def("C.m", "C", DefKind::Function)] },
        ]);
        assert_eq!(names(&tree), vec!["pkg", "b.py"], "folders first");
        let pkg = &tree.children[0];
        assert_eq!(names(pkg), vec!["sub", "a.py"]);
        let a = &pkg.children[1];
        assert_eq!(a.children[0].name, "C");
        assert_eq!(a.children[0].children[0].name, "m", "a method sits under its class");
        assert_eq!(tree.counts(), [3, 3, 1, 2], "counts() includes the root folder itself");
    }

    #[test]
    fn ids_are_unique_even_when_a_name_is_defined_twice() {
        let tree = build("repo", vec![FileDefs { path: "a.py".into(), defs: vec![def("top", "", DefKind::Function), def("top", "", DefKind::Function)] }]);
        let ids: Vec<&str> = tree.children[0].children.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["a.py::top", "a.py::top#2"]);
    }

    #[test]
    fn children_attach_to_the_right_one_of_two_same_named_parents() {
        // class Outer ... class Outer (redefined) with a method in each
        let defs = vec![def("Outer", "", DefKind::Class), def("Outer.m", "Outer", DefKind::Function), def("Outer", "", DefKind::Class), def("Outer.n", "Outer", DefKind::Function)];
        let tree = build("repo", vec![FileDefs { path: "a.py".into(), defs }]);
        let file = &tree.children[0];
        assert_eq!(file.children.len(), 2);
        assert_eq!(names(&file.children[0]), vec!["m"]);
        assert_eq!(names(&file.children[1]), vec!["n"]);
    }

    #[test]
    fn nested_locals_attach_by_parent_qualname() {
        let defs = vec![def("top", "", DefKind::Function), def("top.<locals>.inner", "top", DefKind::Function), def("top.<locals>.inner.<locals>.deepest", "top.<locals>.inner", DefKind::Function), def("after", "", DefKind::Function)];
        let tree = build("repo", vec![FileDefs { path: "a.py".into(), defs }]);
        let file = &tree.children[0];
        assert_eq!(names(file), vec!["top", "after"]);
        assert_eq!(names(&file.children[0]), vec!["inner"]);
        assert_eq!(names(&file.children[0].children[0]), vec!["deepest"]);
    }

    #[test]
    fn a_file_with_an_invalid_repo_path_is_left_out() {
        let tree = build("repo", vec![FileDefs { path: "../escape.py".into(), defs: vec![] }, FileDefs { path: "ok.py".into(), defs: vec![] }]);
        assert_eq!(names(&tree), vec!["ok.py"]);
    }
}
