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
        Ok(Self {
            new_render: None,
            fragment: value.fragment.try_into()?,
            components,
        })
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
            } => Ok(Self { children, statics }),
        }
    }
}

impl FragmentMerge for Root {
    type DiffItem = RootDiff;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        let old_components = self.components.clone();
        let fragment = self.fragment.merge(diff.fragment)?;
        let components = self.components.merge(diff.components)?;

        let mut out = Self {
            new_render: None,
            fragment,
            components,
        };

        out.resolve_components(old_components)?;

        Ok(out)
    }
}

impl Root {
    fn resolve_components(
        &mut self,
        old_components: HashMap<String, Component>,
    ) -> Result<(), MergeError> {
        let new_components = &self.components.clone();

        for component in self.components.values_mut() {
            component.resolve_cids(&old_components, new_components)?
        }

        Ok(())
    }
}

impl Fragment {
    fn resolve_cids(
        &mut self,
        old_components: &HashMap<String, Component>,
        new_components: &HashMap<String, Component>,
    ) -> Result<(), MergeError> {
        let Self::Regular { children, .. } = self else {
            return Ok(());
        };

        for child in children.values_mut() {
            child.resolve_cids(old_components, new_components)?;
        }
        Ok(())
    }
}

impl Child {
    fn resolve_cids(
        &mut self,
        old_components: &HashMap<String, Component>,
        new_components: &HashMap<String, Component>,
    ) -> Result<(), MergeError> {
        match self {
            Child::Fragment(f) => f.resolve_cids(old_components, new_components),
            Child::String(_) => Ok(()),
            Child::ComponentID(id) => {
                let old_id = *id < 0;
                let abs_id = id.abs().to_string();

                let component = if old_id {
                    old_components
                        .get(&abs_id)
                        .ok_or(MergeError::MissingComponent(*id))?
                } else {
                    new_components
                        .get(&abs_id)
                        .ok_or(MergeError::MissingComponent(*id))?
                };

                let mut comp = component.clone();
                comp.resolve_cids(old_components, new_components)?;

                let ComponentStatics::Statics(s) = comp.statics else {
                    return Err(MergeError::UnresolvedComponent);
                };

                *self = Child::Fragment(Fragment::Regular {
                    statics: Some(Statics::Statics(s)),
                    reply: None,
                    children: comp.children,
                });

                Ok(())
            }
        }
    }
}

impl Component {
    fn resolve_cids(
        &mut self,
        old_components: &HashMap<String, Component>,
        new_components: &HashMap<String, Component>,
    ) -> Result<(), MergeError> {
        for child in self.children.values_mut() {
            child.resolve_cids(old_components, new_components)?;
        }

        match self.statics {
            ComponentStatics::ComponentRef(id) => {
                let old_id = id < 0;
                let abs_id = id.abs().to_string();

                let component = if old_id {
                    old_components
                        .get(&abs_id)
                        .ok_or(MergeError::MissingComponent(id))?
                } else {
                    new_components
                        .get(&abs_id)
                        .ok_or(MergeError::MissingComponent(id))?
                };

                let mut comp = component.clone();
                comp.resolve_cids(old_components, new_components)?;
                self.statics = comp.statics
            }
            _ => {
                // raw statics are fine
            }
        };
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
                    reply: current_reply,
                },
                FragmentDiff::UpdateRegular {
                    children: children_diffs,
                    reply: new_reply,
                    ..
                },
            ) => {
                let new_children = current_children.merge(children_diffs)?;
                let new_reply = new_reply.or(current_reply);

                Ok(Self::Regular {
                    children: new_children,
                    statics: current_statics,
                    reply: new_reply,
                })
            }
            (
                Fragment::Comprehension {
                    dynamics: mut current_dynamics,
                    statics,
                    templates: current_templates,
                    stream: current_stream,
                    reply: current_reply,
                },
                FragmentDiff::UpdateComprehension {
                    dynamics: new_dynamics,
                    templates: new_templates,
                    stream: new_stream,
                    reply: new_reply,
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
                Ok(Self::Comprehension {
                    dynamics: current_dynamics,
                    statics,
                    templates,
                    stream,
                    reply: new_reply,
                })
            }
            _ => Err(MergeError::FragmentTypeMismatch),
        }
    }
}
impl FragmentMerge for HashMap<String, Component> {
    type DiffItem = HashMap<String, ComponentDiff>;

    fn merge(self, diff: Self::DiffItem) -> Result<Self, MergeError> {
        let mut new_components: HashMap<String, Component> = HashMap::new();
        for (cid, comp_diff) in diff.into_iter() {
            if let Some(existing) = new_components.get_mut(&cid) {
                *existing = existing.clone().merge(comp_diff)?;
            } else {
                new_components.insert(cid.clone(), comp_diff.try_into()?);
            }
        }

        Ok(new_components)
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
                })
            }
            ComponentDiff::ReplaceCurrent {
                statics, children, ..
            } => Ok(Self { children, statics }),
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
