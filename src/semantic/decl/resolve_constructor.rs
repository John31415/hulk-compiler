use crate::semantic::{
    SemanticAnalyzer,
    types::{ConstructorParam, TypeId},
};

impl SemanticAnalyzer {
    pub fn resolve_constructor_signatures(&mut self) {
        let len = self.ctx.types.infos.len();
        let mut memo: Vec<Option<Vec<ConstructorParam>>> = vec![None; len];
        let mut visiting = vec![false; len];

        for idx in 0..len {
            let type_id = TypeId(idx);
            let effective = self.resolve_effective_constructor(type_id, &mut memo, &mut visiting);
            self.ctx
                .types
                .set_effective_constructor_params(type_id, effective);
        }
    }

    fn resolve_effective_constructor(
        &self,
        type_id: TypeId,
        memo: &mut [Option<Vec<ConstructorParam>>],
        visiting: &mut [bool],
    ) -> Vec<ConstructorParam> {
        if let Some(cached) = &memo[type_id.0] {
            return cached.clone();
        }

        if visiting[type_id.0] {
            return Vec::new();
        }

        visiting[type_id.0] = true;

        let info = &self.ctx.types.infos[type_id.0];

        let effective = match &info.declared_constructor_params {
            Some(params) => params.clone(),
            None => match info.parent {
                Some(parent_id) => self.resolve_effective_constructor(parent_id, memo, visiting),
                None => Vec::new(),
            },
        };

        visiting[type_id.0] = false;
        memo[type_id.0] = Some(effective.clone());
        effective
    }
}
