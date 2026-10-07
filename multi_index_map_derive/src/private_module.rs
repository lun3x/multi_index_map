use syn::visit_mut::{self, VisitMut};
use syn::{parse_quote, ItemMod, Path, Visibility};

/// Preserve the input's scope when generated code moves into a child module.
#[derive(Default)]
pub(crate) struct ParentScope {
    module_depth: usize,
}

pub(crate) fn visibility_in_child(visibility: &Visibility) -> Visibility {
    if matches!(visibility, Visibility::Inherited) {
        return parse_quote!(pub(super));
    }
    let mut visibility = visibility.clone();
    ParentScope::default().visit_visibility_mut(&mut visibility);
    visibility
}

impl VisitMut for ParentScope {
    fn visit_visibility_mut(&mut self, visibility: &mut Visibility) {
        if let Visibility::Restricted(restricted) = visibility {
            self.visit_path_mut(&mut restricted.path);
            // A shifted `pub(super)` can become `pub(in super::super)`.
            if restricted.path.segments.len() > 1 {
                restricted.in_token = Some(Default::default());
            }
        }
    }

    fn visit_item_mod_mut(&mut self, module: &mut ItemMod) {
        for attribute in &mut module.attrs {
            self.visit_attribute_mut(attribute);
        }
        self.visit_visibility_mut(&mut module.vis);
        if let Some((_, items)) = &mut module.content {
            self.module_depth += 1;
            for item in items {
                self.visit_item_mut(item);
            }
            self.module_depth -= 1;
        }
    }

    fn visit_path_mut(&mut self, path: &mut Path) {
        visit_mut::visit_path_mut(self, path);
        if path.leading_colon.is_some() {
            return;
        }
        let parent_count = path
            .segments
            .iter()
            .take_while(|segment| segment.ident == "super")
            .count();
        if parent_count > 0 && parent_count >= self.module_depth {
            // Only paths reaching outside an inline module cross the new scope.
            path.segments.insert(0, parse_quote!(super));
        } else if self.module_depth == 0 {
            if let Some(first) = path
                .segments
                .first_mut()
                .filter(|first| first.ident == "self")
            {
                first.ident = syn::Ident::new("super", first.ident.span());
            }
        }
    }
}
