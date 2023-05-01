use prisma_client_rust_sdk::prisma::{
    prisma_models::{
        walkers::{FieldWalker, RefinedFieldWalker},
        FieldArity,
    },
    psl::parser_database::ScalarFieldType,
};

use crate::generator::prelude::*;

use super::where_params::Variant;

pub fn module(
    field: FieldWalker,
    args: &GenerateArgs,
    module_path: &TokenStream,
) -> ((String, TokenStream), Vec<Variant>) {
    let pcr = quote!(::prisma_client_rust);
    let mut where_param_entries = vec![];

    let field_name = field.name();
    let field_name_pascal = pascal_ident(field_name);
    let field_name_snake = snake_ident(field_name);
    let field_type = field.type_tokens(&quote!(self));

    let set_variant = format_ident!("Set{field_name_pascal}");
    let is_null_variant = format_ident!("{field_name_pascal}IsNull");
    let equals_variant = format_ident!("{field_name_pascal}Equals");

    let arity = field.ast_field().arity;

    let field_module_contents = match field.refine() {
        RefinedFieldWalker::Relation(relation_field) => {
            let relation_model_name_snake = snake_ident(relation_field.related_model().name());

            if let FieldArity::Optional = arity {
                where_param_entries.push(Variant::BaseVariant {
                    definition: quote!(#is_null_variant),
                    match_arm: quote! {
                        Self::#is_null_variant => (
                            #field_name_snake::NAME,
                            #pcr::SerializedWhereValue::Value(#pcr::PrismaValue::Null)
                        )
                    },
                });
            };

            let relation_methods = field.relation_methods().iter().map(|method| {
                let method_action_string = method.to_case(Case::Camel, false);
                let variant_name = format_ident!("{}{}", &field_name_pascal, pascal_ident(method));
                let method_name_snake = snake_ident(method);

                where_param_entries.push(Variant::BaseVariant {
                    definition: quote!(#variant_name(Vec<super::#relation_model_name_snake::WhereParam>)),
                    match_arm: quote! {
                        Self::#variant_name(where_params) => (
                            #field_name_snake::NAME,
                            #pcr::SerializedWhereValue::Object(vec![(
                                #method_action_string.to_string(),
                                #pcr::PrismaValue::Object(
                                    where_params
                                        .into_iter()
                                        .map(#pcr::WhereInput::serialize)
                                        .map(#pcr::SerializedWhereInput::transform_equals)
                                        .collect()
                                ),
                            )])
                        )
                    },
                });

                quote! {
                    pub fn #method_name_snake(value: Vec<#relation_model_name_snake::WhereParam>) -> WhereParam {
                        WhereParam::#variant_name(value)
                    }
                }
            }).collect::<TokenStream>();

            quote! {
                #relation_methods
            }
        }
        RefinedFieldWalker::Scalar(scalar_field) => {
            if let ScalarFieldType::CompositeType(cf_id) = scalar_field.scalar_field_type() {
                let comp_type = field.db.walk(cf_id);

                let comp_type_snake = snake_ident(comp_type.name());

                // Filters

                let optional_filters = arity
                    .is_optional()
                    .then(|| {
                        let is_set_filter = {
                            let where_param_variant = format_ident!("{field_name_pascal}IsSet");

                            where_param_entries.push(Variant::BaseVariant {
                                definition: quote!(#where_param_variant),
                                match_arm: quote! {
                                    Self::#where_param_variant => (
                                        #field_name_snake::NAME,
                                        #pcr::SerializedWhereValue::Value(
                                            #pcr::PrismaValue::Boolean(true)
                                        )
                                     )
                                },
                            });

                            quote! {
                                pub fn is_set() -> WhereParam {
                                    WhereParam::#where_param_variant
                                }
                            }
                        };

                        vec![is_set_filter]
                    })
                    .unwrap_or(vec![]);

                let many_filters: Vec<_> = arity
                    .is_list()
                    .then(|| {
                        let equals_filter = {
                            let where_param_variant = format_ident!("{field_name_pascal}Equals");
                            let content_type =
                                quote!(Vec<#module_path::#comp_type_snake::WhereParam>);

                            where_param_entries.push(Variant::BaseVariant {
	                            definition: quote!(#where_param_variant(Vec<#content_type>)),
	                            match_arm: quote! {
	                                Self::#where_param_variant(where_params) => (
	                                    #field_name_snake::NAME,
	                                    #pcr::SerializedWhereValue::Object(vec![(
	                                        "equals".to_string(),
	                                        #pcr::PrismaValue::List(
	                                        	where_params
	                                         		.into_iter()
	                                           		.map(|params|
		                                         		#pcr::PrismaValue::Object(
				                                            params
				                                                .into_iter()
				                                                .map(#pcr::WhereInput::serialize)
				                                                .map(#pcr::SerializedWhereInput::transform_equals)
				                                                .collect()
		                                           		)
		                                         	)
													.collect()
	                                        )
	                                    )])
	                                )
	                            },
                            });

                            quote! {
                                pub fn equals(params: Vec<#content_type>) -> WhereParam {
                                    WhereParam::#where_param_variant(params)
                                }
                            }
                        };

                        let is_empty_filter = {
                            let where_param_variant = format_ident!("{field_name_pascal}IsEmpty");

                            where_param_entries.push(Variant::BaseVariant {
                                definition: quote!(#where_param_variant),
                                match_arm: quote! {
                                    Self::#where_param_variant => (
                                        #field_name_snake::NAME,
                                        #pcr::SerializedWhereValue::Object(vec![(
                                            "isEmpty".to_string(),
                                            #pcr::PrismaValue::Boolean(true)
                                        )])
                                    )
                                },
                            });

                            quote! {
                                pub fn is_empty() -> WhereParam {
                                    WhereParam::#where_param_variant
                                }
                            }
                        };

                        let general_filters = ["every", "some", "none"].iter().map(|method| {
                            let method_snake = snake_ident(method);
                            let method_pascal = pascal_ident(method);

                            let where_param_variant =
                                format_ident!("{field_name_pascal}{method_pascal}");
                            let content_type =
                                quote!(Vec<#module_path::#comp_type_snake::WhereParam>);

                            where_param_entries.push(Variant::BaseVariant {
                                definition: quote!(#where_param_variant(#content_type)),
                                match_arm: quote! {
                                Self::#where_param_variant(where_params) => (
                                    #field_name_snake::NAME,
                                    #pcr::SerializedWhereValue::Object(vec![(
                                        #method.to_string(),
                                        #pcr::PrismaValue::Object(
                                            where_params
                                                .into_iter()
                                                .map(#pcr::WhereInput::serialize)
                                                .map(#pcr::SerializedWhereInput::transform_equals)
                                                .collect()
                                        )
                                    )])
                                    )
                                },
                            });

                            quote! {
                                pub fn #method_snake(params: #content_type) -> WhereParam {
                                    WhereParam::#where_param_variant(params)
                                }
                            }
                        });

                        general_filters
                            .chain([equals_filter, is_empty_filter])
                            .collect()
                    })
                    .unwrap_or_else(|| {
                         ["equals", "is", "isNot"]
                        .iter()
                        .map(|method| {
                            let method_snake = snake_ident(method);
                            let method_pascal = pascal_ident(method);

                            let where_param_variant =
                                format_ident!("{field_name_pascal}{method_pascal}");
                            let content_type =
                                quote!(Vec<#module_path::#comp_type_snake::WhereParam>);

                            where_param_entries.push(Variant::BaseVariant {
                                definition: quote!(#where_param_variant(#content_type)),
                                match_arm: quote! {
                                    Self::#where_param_variant(where_params) => (
                                        #field_name_snake::NAME,
                                        #pcr::SerializedWhereValue::Object(vec![(
                                            #method.to_string(),
                                            #pcr::PrismaValue::Object(
                                                where_params
                                                    .into_iter()
                                                    .map(#pcr::WhereInput::serialize)
                                                    .map(#pcr::SerializedWhereInput::transform_equals)
                                                    .collect()
                                            )
                                        )])
                                     )
                                },
                            });

                            quote! {
                                pub fn #method_snake(params: #content_type) -> WhereParam {
                                    WhereParam::#where_param_variant(params)
                                }
                            }
                        })
                        .collect()

                    });

                // Writes

                let set_fn = comp_type
                    .fields()
                    .filter(|f| f.required_on_create())
                    .map(|field| {
                        field.type_tokens(module_path)?;
                        Some(field)
                    })
                    .collect::<Option<Vec<_>>>()
                    .map(|_| {
                        let create_struct = arity.wrap_type(&quote!(#comp_type_snake::Create));

                        quote! {
                            pub struct Set(#create_struct);

                            pub fn set<T: From<Set>>(create: #create_struct) -> T {
                                Set(create).into()
                            }

                            impl From<Set> for SetParam {
                                fn from(Set(create): Set) -> Self {
                                     SetParam::#set_variant(create)
                                }
                            }

                            impl From<Set> for UncheckedSetParam {
                                fn from(Set(create): Set) -> Self {
                                     UncheckedSetParam::#field_name_pascal(create)
                                }
                            }
                        }
                    });

                let unset_fn = arity.is_optional().then(|| {
                    let set_param_variant = format_ident!("Unset{field_name_pascal}");

                    quote! {
                        pub fn unset() -> SetParam {
                            SetParam::#set_param_variant
                        }
                    }
                });
                let update_fn = (!arity.is_list()).then(|| {
                    let set_param_variant = format_ident!("Update{field_name_pascal}");

                    quote! {
                        pub fn update(params: Vec<#comp_type_snake::SetParam>) -> SetParam {
                            SetParam::#set_param_variant(params)
                        }
                    }
                });
                let upsert_fn = arity.is_optional().then(|| {
                    let set_param_variant = format_ident!("Upsert{field_name_pascal}");

                    quote! {
                        pub fn upsert(
                            create: #comp_type_snake::Create,
                            update: Vec<#comp_type_snake::SetParam>
                        ) -> SetParam {
                            SetParam::#set_param_variant(create, update)
                        }
                    }
                });
                let push_fn = arity.is_list().then(|| {
                    let set_param_variant = format_ident!("Push{field_name_pascal}");

                    quote! {
                        pub fn push(creates: Vec<#comp_type_snake::Create>) -> SetParam {
                            SetParam::#set_param_variant(creates)
                        }
                    }
                });
                let update_many_fn = arity.is_list().then(|| {
                    let set_param_variant = format_ident!("UpdateMany{field_name_pascal}");

                    quote! {
                        pub fn update_many(
                            _where: Vec<#comp_type_snake::WhereParam>,
                            update: Vec<#comp_type_snake::SetParam>
                        ) -> SetParam {
                            SetParam::#set_param_variant(_where, update)
                        }
                    }
                });
                let delete_many_fn = arity.is_list().then(|| {
                    let set_param_variant = format_ident!("DeleteMany{field_name_pascal}");

                    quote! {
                        pub fn delete_many(
                            _where: Vec<#comp_type_snake::WhereParam>,
                        ) -> SetParam {
                            SetParam::#set_param_variant(_where)
                        }
                    }
                });

                let order_by_fn = (!arity.is_list()).then(|| {
                    quote! {
                        pub fn order(params: Vec<#comp_type_snake::OrderByParam>) -> OrderByParam {
                            OrderByParam::#field_name_pascal(params)
                        }
                    }
                });

                quote! {
                    #(#optional_filters)*
                    #(#many_filters)*

                    #set_fn
                    #unset_fn
                    #update_fn
                    #upsert_fn
                    #push_fn
                    #update_many_fn
                    #delete_many_fn

                    #order_by_fn
                }
            } else {
                let read_fns = args.read_filter(scalar_field).map(|read_filter| {
                    let filter_enum = format_ident!("{}Filter", &read_filter.name);

                    let model = field.model();

                    // Add equals query functions. Unique/Where enum variants are added in unique/primary key sections earlier on.
                    let equals = match (
                        scalar_field.is_single_pk(),
                        model.indexes().any(|idx| {
                            let mut fields = idx.fields();
                            idx.is_unique() && fields.len() == 1 && fields.next().map(|f| f.field_id()) == Some(scalar_field.field_id())
                        }),
                        arity.is_required()
                    ) {
                        (true, _, _) | (_, true, true) => quote! {
                            pub fn equals<T: From<UniqueWhereParam>>(value: #field_type) -> T {
                                UniqueWhereParam::#equals_variant(value).into()
                            }
                        },
                        (_, true, false) => quote! {
                            pub fn equals<A, T: #pcr::FromOptionalUniqueArg<Set, Arg = A>>(value: A) -> T {
                                T::from_arg(value)
                            }
                        },
                        (_, _, _) => quote! {
                            pub fn equals(value: #field_type) -> WhereParam {
                                WhereParam::#field_name_pascal(_prisma::read_filters::#filter_enum::Equals(value))
                            }
                        }
                    };

                    where_param_entries.push(Variant::BaseVariant {
                        definition: quote!(#field_name_pascal(super::_prisma::read_filters::#filter_enum)),
                        match_arm: quote! {
                            Self::#field_name_pascal(value) => (
                                #field_name_snake::NAME,
                                value.into()
                            )
                        },
                    });

                    let read_methods = read_filter.methods.iter().filter_map(|method| {
                        if method.name == "Equals" { return None }

                        let method_name_snake = snake_ident(&method.name);
                        let method_name_pascal = pascal_ident(&method.name);

                        let typ = method.type_tokens(&quote!(super::super), &args.schema.db);

                        Some(quote!(fn #method_name_snake(_: #typ) -> #method_name_pascal;))
                    });

                    quote! {
                        #equals

                        #pcr::scalar_where_param_fns!(
                            _prisma::read_filters::#filter_enum,
                            #field_name_pascal,
                            { #(#read_methods)* }
                        );
                    }
                });

                quote! {
                    #read_fns
                }
            }
        }
    };

    (
        (field_name.to_string(), field_module_contents),
        where_param_entries,
    )
}
