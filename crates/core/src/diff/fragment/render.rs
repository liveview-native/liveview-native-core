use super::*;

// This is to render the Root as an XML tree in String form.
impl TryInto<String> for Root {
    type Error = RenderError;

    fn try_into(self) -> Result<String, Self::Error> {
        let mut out = String::new();
        let inner = self.fragment.render(&self.components, None, None)?;
        out.push_str(&inner);
        Ok(out)
    }
}

impl Fragment {
    pub fn render(
        &self,
        components: &HashMap<String, Component>,
        cousin_statics: Option<Vec<String>>,
        parent_templates: Templates,
    ) -> Result<String, RenderError> {
        let mut out = String::new();
        match &self {
            Fragment::Regular {
                children, statics, ..
            } => {
                match statics {
                    None => {}
                    Some(Statics::String(_)) => {}
                    Some(Statics::Statics(statics)) => {
                        out.push_str(&statics[0]);
                        // We start at index 1 rather than zero here because
                        // templates and statics are suppose to wrap the inner
                        // contents of the children.
                        for (i, static_item) in statics.iter().enumerate().skip(1) {
                            if let Some(child) = children.get(&(i - 1).to_string()) {
                                let val = child.render(
                                    components,
                                    cousin_statics.clone(),
                                    parent_templates.clone(),
                                )?;
                                out.push_str(&val);
                            }
                            out.push_str(static_item);
                        }
                    }
                    Some(Statics::TemplateRef(template_id)) => {
                        let templates = parent_templates.ok_or(RenderError::NoTemplates)?;
                        let template = templates
                            .get(&(template_id.to_string()))
                            .ok_or(RenderError::TemplateNotFound(*template_id))?;
                        out.push_str(&template[0]);
                        // We start at index 1 rather than zero here because
                        // templates and statics are suppose to wrap the inner
                        // contents of the children.
                        for (i, template_item) in template.iter().enumerate().skip(1) {
                            let child_id = i - 1;
                            let child = children
                                .get(&child_id.to_string())
                                .ok_or(RenderError::ChildNotFoundForTemplate(child_id as i32))?;
                            let val = child.render(
                                components,
                                cousin_statics.clone(),
                                Some(templates.clone()),
                            )?;
                            out.push_str(&val);
                            out.push_str(template_item);
                        }
                    }
                }
            }
            Fragment::Comprehension {
                dynamics,
                statics,
                templates,
                ..
            } => {
                let templates: Templates = match (parent_templates, templates) {
                    (None, None) => None,
                    (None, Some(t)) => Some(t.clone()),
                    (Some(t), None) => Some(t),
                    (Some(parent), Some(child)) => Some(parent).merge(Some(child.clone()))?,
                };
                match (statics, cousin_statics) {
                    (None, None) => {
                        for children in dynamics.iter() {
                            for child in children.iter() {
                                let val = child.render(components, None, templates.clone())?;
                                out.push_str(&val);
                            }
                        }
                    }
                    (None, Some(statics)) => {
                        for children in dynamics.iter() {
                            out.push_str(&statics[0]);
                            // We start at index 1 rather than zero here because
                            // templates and statics are suppose to wrap the inner
                            // contents of the children.
                            for i in 1..statics.len() {
                                let child = &children[i - 1];

                                let val = child.render(components, None, templates.clone())?;
                                out.push_str(&val);
                                out.push_str(&statics[i]);
                            }
                        }
                    }
                    (Some(statics), None) => {
                        match statics {
                            Statics::String(_) => {}
                            Statics::Statics(statics) => {
                                for children in dynamics.iter() {
                                    out.push_str(&statics[0]);
                                    // We start at index 1 rather than zero here because
                                    // templates and statics are suppose to wrap the inner
                                    // contents of the children.
                                    for i in 1..statics.len() {
                                        let child = &children[i - 1];

                                        let val =
                                            child.render(components, None, templates.clone())?;
                                        out.push_str(&val);
                                        out.push_str(&statics[i]);
                                    }
                                }
                            }
                            Statics::TemplateRef(template_id) => {
                                if let Some(ref this_template) = templates {
                                    if let Some(template_statics) =
                                        this_template.get(&template_id.to_string())
                                    {
                                        for children in dynamics.iter() {
                                            out.push_str(&template_statics[0]);

                                            // We start at index 1 rather than zero here because
                                            // templates and statics are suppose to wrap the inner
                                            // contents of the children.
                                            for i in 1..template_statics.len() {
                                                let child = &children[i - 1];

                                                let val = child.render(
                                                    components,
                                                    None,
                                                    templates.clone(),
                                                )?;
                                                out.push_str(&val);
                                                out.push_str(&template_statics[i]);
                                            }
                                        }
                                    } else {
                                        return Err(RenderError::TemplateNotFound(*template_id));
                                    }
                                } else {
                                    return Err(RenderError::NoTemplates);
                                }
                            }
                        }
                    }
                    (Some(_statics), Some(_cousin_templates)) => {
                        panic!("Either statics or cousin statics but not both");
                    }
                }
            }
        }
        Ok(out)
    }
}

impl Child {
    pub fn render(
        &self,
        components: &HashMap<String, Component>,
        statics: Option<Vec<String>>,
        templates: Templates,
    ) -> Result<String, RenderError> {
        match self {
            Child::Fragment(fragment) => fragment.render(components, statics, templates),
            Child::ComponentID(cid) => {
                if let Some(component) = components.get(&cid.to_string()) {
                    component.render(components)
                } else {
                    Err(RenderError::ComponentNotFound(*cid))
                }
            }
            Child::String(OneOrManyStrings::One(s)) => Ok(s.clone()),
            Child::String(OneOrManyStrings::Many(s)) => Ok(s.concat()),
        }
    }
}

impl Component {
    pub fn render(&self, components: &HashMap<String, Component>) -> Result<String, RenderError> {
        match &self.statics {
            ComponentStatics::Statics(statics) => {
                let mut out = String::new();

                out.push_str(&statics[0]);
                // We start at index 1 rather than zero here because
                // templates and statics are suppose to wrap the inner
                // contents of the children.
                for (i, static_item) in statics.iter().enumerate().skip(1) {
                    let inner = self
                        .children
                        .get(&(i - 1).to_string())
                        .ok_or(RenderError::ChildNotFoundForStatic((i - 1) as i32))?;
                    let val = inner.render(components, None, None)?;
                    out.push_str(&val);
                    out.push_str(static_item);
                }
                Ok(out)
            }

            ComponentStatics::ComponentRef(mut cid) => {
                let outer_statics: Vec<String>;
                let cousin_component: Component;
                loop {
                    if let Some(component) = components.get(&cid.to_string()) {
                        match &component.statics {
                            ComponentStatics::Statics(s) => {
                                outer_statics = s.to_vec();
                                cousin_component = component.clone();
                                break;
                            }
                            ComponentStatics::ComponentRef(bread_crumb_cid) => {
                                cid = *bread_crumb_cid;
                            }
                        }
                    } else {
                        return Err(RenderError::ComponentNotFound(cid));
                    }
                }
                let mut out = String::new();

                out.push_str(&outer_statics[0]);
                // We start at index 1 rather than zero here because
                // templates and statics are suppose to wrap the inner
                // contents of the children.
                for (i, outer_static_item) in outer_statics.iter().enumerate().skip(1) {
                    let child = self
                        .children
                        .get(&(i - 1).to_string())
                        .ok_or(RenderError::ChildNotFoundForStatic((i - 1) as i32))?;

                    let cousin = cousin_component
                        .children
                        .get(&(i - 1).to_string())
                        .ok_or(RenderError::CousinNotFound((i - 1) as i32))?;

                    let val = child.render(components, cousin.statics(), None)?;
                    out.push_str(&val);
                    out.push_str(outer_static_item);
                }
                Ok(out)
            }
        }
    }
}
