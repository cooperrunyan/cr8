use std::collections::HashMap;

use crate::compiler::ast::{AddrByte, AstNode, Ident, Instruction, MacroArg, ToNode, Value};

use super::Compiler;

impl Compiler {
    pub(crate) fn resolve_macros(&mut self) {
        let mut new_tree = vec![];

        let mut tree = vec![];
        tree.append(&mut self.tree);

        for node in tree {
            let mut stripped = self.fill_macro(node);
            new_tree.append(&mut stripped);
        }

        self.tree = new_tree;
    }

    fn fill_macro(&self, node: AstNode) -> Vec<AstNode> {
        let mut tree = vec![];

        match node {
            AstNode::Instruction(Instruction::Macro(mac_name, args)) => {
                use MacroArg as MA;
                use Value as V;
                let mac = match self.macros.get(&mac_name) {
                    Some(m) => m,
                    None => panic!("Macro '{mac_name}' not defined"),
                };

                let mut captured_args: HashMap<String, Value> = HashMap::new();

                for capturer in mac.captures.iter() {
                    if capturer.args.len() != args.len() {
                        continue;
                    }

                    let mut valid = true;

                    macro_rules! invalid {
                        () => {{
                            valid = false;
                            break;
                        }};
                    }

                    macro_rules! insert {
                        ($n:expr, $t:ident($v:expr)) => {{
                            captured_args.insert($n.to_string(), V::$t($v.clone()));
                        }};
                    }

                    macro_rules! insert_addr {
                        ($n:expr, $f:expr, $r:expr) => {{
                            captured_args.insert($n.to_string(), $f);
                            captured_args.insert(
                                format!("{}.l", $n),
                                V::AddrByte(AddrByte::Low($r.clone())),
                            );
                            captured_args.insert(
                                format!("{}.h", $n),
                                V::AddrByte(AddrByte::High($r.clone())),
                            );
                        }};
                    }

                    for (i, capture_arg) in capturer.args.iter().enumerate() {
                        let current = args.get(i).unwrap();
                        match capture_arg {
                            MA::Immediate(name) => match current {
                                V::Immediate(v) => insert!(name, Immediate(v)),
                                V::Ident(id) => insert!(name, Ident(id)),
                                V::Expression(e) => insert!(name, Expression(e)),
                                _ => invalid!(),
                            },
                            MA::Register(name) => match current {
                                V::Register(r) => insert!(name, Register(r)),
                                _ => invalid!(),
                            },
                            MA::ImmReg(name) => match current {
                                V::Immediate(v) => insert!(name, Immediate(v)),
                                V::Register(r) => insert!(name, Register(r)),
                                V::Ident(id) => insert!(name, Ident(id)),
                                _ => invalid!(),
                            },
                            MA::Addr(name) => match current {
                                V::Ident(i) => match i {
                                    Ident::Static(a) | Ident::Addr(a) => {
                                        insert_addr!(name, V::Ident(Ident::Addr(a.to_owned())), a);
                                    }
                                    _ => invalid!(),
                                },
                                V::Expression(e) => {
                                    insert_addr!(name, V::Expression(e.clone()), e);
                                }
                                _ => invalid!(),
                            },
                        }
                    }

                    if !valid {
                        continue;
                    }

                    for instruction in capturer.body.iter() {
                        let (empty, args) = match instruction {
                            Instruction::Macro(m, args) => {
                                (Instruction::Macro(m.to_string(), vec![]), args)
                            }
                            Instruction::Native(n, args) => {
                                (Instruction::Native(n.to_owned(), vec![]), args)
                            }
                        };
                        let mut new_args: Vec<Value> = vec![];

                        for arg in args {
                            match arg {
                                V::Ident(Ident::MacroArg(ma)) => {
                                    let Some(val) = captured_args.get(ma) else {
                                        panic!("Attempted to use undefined macro arg at {mac_name:#?} {empty:#?}");
                                    };
                                    new_args.push(val.to_owned());
                                }
                                oth => new_args.push(oth.clone()),
                            }
                        }

                        match instruction {
                            Instruction::Macro(m, _) => {
                                let mut nodes = self.fill_macro(
                                    Instruction::Macro(m.to_owned(), new_args).to_node(),
                                );
                                tree.append(&mut nodes);
                            }
                            Instruction::Native(n, _) => {
                                tree.push(Instruction::Native(n.to_owned(), new_args).to_node())
                            }
                        };
                    }
                    return tree;
                }
            }
            _ => tree.push(node),
        };

        tree
    }
}
