use super::*;

pub trait FragmentMerge: Sized {
    type DiffItem;
    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError>;
}

// This is a direct conversion from RootDiff to Root.
impl TryFrom<RootDiff> for Root {
    type Error = MergeError;
    fn try_from(value: RootDiff) -> Result<Self, MergeError> {
        let mut components: HashMap<String, Component> = HashMap::new();

        for (key, value) in value.components.into_iter() {
            components.insert(key, value.try_into()?);
        }

        let fragment = value.fragment.try_into()?;

        Root::new(fragment, HashMap::new(), components)
    }
}

impl TryFrom<ChildDiff> for Child {
    type Error = MergeError;

    fn try_from(value: ChildDiff) -> Result<Self, Self::Error> {
        match value {
            ChildDiff::String(s) => Ok(Child::String(s)),
            ChildDiff::ComponentID(cid) => Ok(Child::ComponentID(cid)),
            ChildDiff::Fragment(fragment_diff) => Ok(Self::Fragment(fragment_diff.try_into()?)),
        }
    }
}

impl TryFrom<ComponentDiff> for Component {
    type Error = MergeError;
    fn try_from(value: ComponentDiff) -> Result<Self, MergeError> {
        match value {
            ComponentDiff::UpdateRegular { .. } => Err(MergeError::CreateComponentFromUpdate),
            ComponentDiff::ReplaceCurrent {
                children, statics, ..
            } => Ok(Self {
                children,
                statics,
                is_root: None,
            }),
        }
    }
}

impl FragmentMerge for Root {
    type DiffItem = RootDiff;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        let old_components = self.components.clone();
        let fragment = self.fragment.merge(diff.fragment)?;
        let new_components = self.components.merge(diff.components)?;

        Root::new(fragment, old_components, new_components)
    }
}

impl Root {
    /// create a new root, resolving the CID's
    pub fn new(
        fragment: Fragment,
        old_components: HashMap<String, Component>,
        new_components: HashMap<String, Component>,
    ) -> Result<Self, MergeError> {
        let mut out = Self {
            new_render: None,
            fragment,
            components: new_components,
        };

        out.resolve_components(old_components)?;
        Ok(out)
    }

    fn resolve_components(
        &mut self,
        old_components: HashMap<String, Component>,
    ) -> Result<(), MergeError> {
        let new_components = &self.components.clone();

        let ctx = ResolveCtx {
            old_components: &old_components,
            new_components,
        };

        for component in self.components.values_mut() {
            component.resolve_cids(&ctx)?
        }

        Ok(())
    }
}

pub struct ResolveCtx<'a> {
    old_components: &'a HashMap<String, Component>,
    new_components: &'a HashMap<String, Component>,
}

impl ResolveCtx<'_> {
    pub fn get(&self, cid: i32) -> Result<&Component, MergeError> {
        let old_id = cid < 0;
        let abs_id = cid.abs().to_string();

        if old_id {
            self.old_components
                .get(&abs_id)
                .ok_or(MergeError::MissingComponent(cid))
        } else {
            self.new_components
                .get(&abs_id)
                .ok_or(MergeError::MissingComponent(cid))
        }
    }
}

impl Component {
    fn resolve_cids(&mut self, ctx: &ResolveCtx) -> Result<(), MergeError> {
        match self.statics {
            ComponentStatics::ComponentRef(id) => {
                let comp = ctx.get(id)?.clone();

                // currently the spec states that components should
                // be merged and resolved by copying statics from the source tree
                // // https://github.com/phoenixframework/phoenix_live_view/blob/93d242460f5222b1d89e54df56624bc96d53d659/assets/js/phoenix_live_view/rendered.js#L238
                self.statics = comp.statics;

                // then we merge the component ID tree
                // using the scheme here
                // https://github.com/phoenixframework/phoenix_live_view/blob/93d242460f5222b1d89e54df56624bc96d53d659/assets/js/phoenix_live_view/rendered.js#L238
                for (id, new_child) in comp.children {
                    match self.children.get_mut(&id) {
                        Some(old_child) => old_child.merge_component_trees(new_child)?,
                        None => {
                            self.children.insert(id, new_child);
                        }
                    }
                }
            }
            _ => {
                // raw statics are fine
            }
        };
        Ok(())
    }
}

impl Child {
    fn merge_component_trees(&mut self, other: Self) -> Result<(), MergeError> {
        let (old_frag, new_frag) = match (self, other) {
            (Child::Fragment(old), Child::Fragment(new)) => (old, new),
            (Child::ComponentID(_), _) | (_, Child::ComponentID(_)) => {
                return Err(MergeError::UnresolvedComponent)
            }
            (_, Child::String(_)) | (Child::String(_), _) => return Ok(()),
        };
        match (old_frag, new_frag) {
            (
                Fragment::Comprehension { statics, .. },
                Fragment::Comprehension {
                    statics: new_statics,
                    ..
                },
            ) => {
                if statics.is_none() {
                    *statics = new_statics;
                }
            }
            (
                Fragment::Regular {
                    statics, children, ..
                },
                Fragment::Regular {
                    statics: new_statics,
                    children: new_children,
                    ..
                },
            ) => {
                if statics.is_none() {
                    *statics = new_statics;
                }

                for (id, new_child) in new_children {
                    match children.get_mut(&id) {
                        Some(old_child) => old_child.merge_component_trees(new_child)?,
                        None => {
                            children.insert(id, new_child);
                        }
                    }
                }
            }
            _ => return Err(MergeError::FragmentTypeMismatch),
        }
        Ok(())
    }
}

impl FragmentDiff {
    fn should_replace_current(&self) -> bool {
        match self {
            FragmentDiff::UpdateRegular { statics, .. }
            | FragmentDiff::UpdateComprehension { statics, .. } => statics.is_some(),
        }
    }
}

impl TryFrom<Vec<StreamAttribute>> for Stream {
    type Error = StreamConversionError;
    fn try_from(stream_attrs: Vec<StreamAttribute>) -> Result<Self, Self::Error> {
        let mut stream: Stream = Stream {
            id: String::new(),
            stream_items: Vec::new(),
        };
        for stream_attr in &stream_attrs {
            match stream_attr {
                StreamAttribute::StreamID(id) => {
                    stream.id = id.to_string();
                }
                StreamAttribute::Inserts(inserts) => {
                    for (stream_id, index, limit) in inserts.iter() {
                        stream.stream_items.push(StreamItem {
                            id: stream_id.to_string(),
                            index: *index,
                            limit: *limit,
                        });
                    }
                }
                StreamAttribute::DeleteIDs(_delete_ids) => {
                    log::error!("Deleting Stream IDs when converting from a fragmentdiff to a fragment should not occur");
                }
                StreamAttribute::ResetStream(reset) => {
                    if *reset {
                        stream.stream_items = Vec::new();
                    }
                }
            }
        }
        Ok(stream)
    }
}

impl FragmentMerge for Fragment {
    type DiffItem = FragmentDiff;

    fn merge(self, diff: FragmentDiff) -> Result<Self, MergeError> {
        if diff.should_replace_current() {
            return diff.try_into();
        }

        match (self, diff) {
            (
                Fragment::Regular {
                    children: current_children,
                    statics: current_statics,
                    is_root: current_reply,
                    ..
                },
                FragmentDiff::UpdateRegular {
                    children: children_diffs,
                    is_root: new_reply,
                    ..
                },
            ) => {
                let new_children = current_children.merge(children_diffs)?;
                let new_reply = new_reply.or(current_reply);
                let new_render = new_reply.map(|i| i != 0);

                Ok(Self::Regular {
                    children: new_children,
                    statics: current_statics,
                    is_root: new_reply,
                    new_render,
                })
            }
            (
                Fragment::Comprehension {
                    dynamics: mut current_dynamics,
                    statics,
                    templates: current_templates,
                    stream: current_stream,
                    is_root: current_reply,
                    ..
                },
                FragmentDiff::UpdateComprehension {
                    dynamics: new_dynamics,
                    templates: new_templates,
                    stream: new_stream,
                    is_root: new_reply,
                    ..
                },
            ) => {
                let new_reply = new_reply.or(current_reply);
                let templates = current_templates.merge(new_templates)?;

                let new_dynamics: Vec<Vec<Child>> = new_dynamics
                    .into_iter()
                    .map(|children_children| {
                        children_children
                            .into_iter()
                            .map(|child| child.try_into())
                            .collect::<Result<Vec<Child>, MergeError>>()
                    })
                    .collect::<Result<Vec<Vec<Child>>, MergeError>>()?;

                let stream = match (current_stream, new_stream) {
                    (None, None) => {
                        current_dynamics = new_dynamics;
                        None
                    }
                    (None, Some(stream_attrs)) => Some(Stream::try_from(stream_attrs)?),
                    (Some(stream), None) => Some(stream),
                    (Some(mut stream), Some(stream_update)) => {
                        for stream_attr in &stream_update {
                            match stream_attr {
                                StreamAttribute::StreamID(stream_id) => {
                                    if stream.id != *stream_id {
                                        return Err(MergeError::StreamIDMismatch);
                                    }
                                }
                                StreamAttribute::Inserts(inserts) => {
                                    for (insert_id, index, _limit) in inserts.iter() {
                                        if let Some(dynamic) =
                                            new_dynamics.iter().find(|children| {
                                                children.iter().any(|child| {
                                                    Child::String(
                                                        format!(" id=\"{insert_id}\"").into(),
                                                    ) == *child
                                                })
                                            })
                                        {
                                            if *index == -1 {
                                                current_dynamics.push(dynamic.clone());
                                            }
                                        }
                                    }
                                }
                                StreamAttribute::DeleteIDs(delete_ids) => {
                                    for delete_id in delete_ids {
                                        if let Some(index) =
                                            current_dynamics.iter().position(|children| {
                                                children.iter().any(|child| {
                                                    Child::String(
                                                        format!(" id=\"{delete_id}\"").into(),
                                                    ) == *child
                                                })
                                            })
                                        {
                                            current_dynamics.remove(index);
                                        }
                                    }
                                }
                                StreamAttribute::ResetStream(reset) => {
                                    if *reset {
                                        stream.stream_items = Vec::new();
                                        current_dynamics.clone_from(&new_dynamics)
                                    }
                                }
                            }
                        }
                        Some(stream)
                    }
                };

                let new_render = new_reply.map(|i| i != 0);

                Ok(Self::Comprehension {
                    dynamics: current_dynamics,
                    statics,
                    templates,
                    stream,
                    is_root: new_reply,
                    new_render,
                })
            }
            _ => Err(MergeError::FragmentTypeMismatch),
        }
    }
}
impl FragmentMerge for HashMap<String, Component> {
    type DiffItem = HashMap<String, ComponentDiff>;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        let mut components = self;
        for (cid, comp_diff) in diff.into_iter() {
            if let Some(existing) = components.get_mut(&cid) {
                *existing = existing.clone().merge(comp_diff)?;
            } else {
                components.insert(cid.clone(), comp_diff.try_into()?);
            }
        }

        Ok(components)
    }
}

impl FragmentMerge for Component {
    type DiffItem = ComponentDiff;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        match diff {
            ComponentDiff::UpdateRegular {
                children: children_diffs,
                ..
            } => {
                let new_children = self.children.merge(children_diffs)?;
                Ok(Self {
                    children: new_children,
                    statics: self.statics,
                    is_root: None,
                })
            }
            ComponentDiff::ReplaceCurrent {
                statics, children, ..
            } => Ok(Self {
                children,
                statics,
                is_root: None,
            }),
        }
    }
}

impl FragmentMerge for Templates {
    type DiffItem = Templates;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        match (self, diff) {
            (None, None) => Ok(None),
            (None, Some(template)) => Ok(Some(template)),
            (Some(template), None) => Ok(Some(template)),
            (Some(mut current), Some(new)) => {
                for (key, val) in new.into_iter() {
                    current.insert(key, val);
                }
                Ok(Some(current))
            }
        }
    }
}
impl FragmentMerge for Child {
    type DiffItem = ChildDiff;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        match (self, diff) {
            (Child::Fragment(current_fragment), ChildDiff::Fragment(fragment_diff)) => {
                Ok(Self::Fragment(current_fragment.merge(fragment_diff)?))
            }
            (_, ChildDiff::String(s)) => Ok(Self::String(s)),
            (_, ChildDiff::ComponentID(id)) => Ok(Self::ComponentID(id)),
            (_, ChildDiff::Fragment(fragment_diff)) => {
                Ok(Self::Fragment(fragment_diff.try_into()?))
            }
        }
    }
}

impl FragmentMerge for HashMap<String, Child> {
    type DiffItem = HashMap<String, ChildDiff>;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        let mut new_children = self;
        for (index, comp_diff) in diff.into_iter() {
            if let Some(child) = new_children.get_mut(&index) {
                *child = child.clone().merge(comp_diff)?;
            } else {
                new_children.insert(index, comp_diff.try_into()?);
            }
        }
        Ok(new_children)
    }
}
