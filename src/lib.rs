use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Meta, MetaList, NestedMeta, Type, TypePath, Variant};

#[proc_macro_derive(EnumFromImpler, attributes(impl_from))]
pub fn create_enum_from_impls(input: TokenStream) -> TokenStream {
	let ast: DeriveInput = syn::parse(input).unwrap();

	let enum_name = &ast.ident;

	let data = match ast.data {
		Data::Enum(data) => data,
		_ => panic!("Only works on enums"),
	};

	let has_impl_all = ast.attrs.iter().any(|attr| match attr.parse_meta() {
		Ok(Meta::Path(ref path)) => path
			.get_ident()
			.map(|v| v == "impl_from")
			.unwrap_or_default(),
		_ => false,
	});

	let mut impls = Vec::with_capacity(ast.attrs.len());

	for variant in data.variants {
		let variant: Variant = variant;
		let foreign_type = variant
			.attrs
			.iter()
			.find_map(|attr| match attr.parse_meta() {
				Ok(Meta::Path(ref path)) => {
					if !path
						.get_ident()
						.map(|v| v == "impl_from")
						.unwrap_or_default()
					{
						return None;
					}
					unnamed_variant_type(&variant)
				}
				Ok(Meta::List(MetaList { path, nested, .. })) => {
					if !path
						.get_ident()
						.map(|v| v == "impl_from")
						.unwrap_or_default()
					{
						return None;
					}
					match nested.into_iter().next() {
						Some(NestedMeta::Meta(Meta::Path(path))) => {
							Some(Type::Path(TypePath { qself: None, path }))
						}
						_ => None,
					}
				}
				_ => None,
			});

		let foreign_type = match foreign_type {
			Some(v) => v,
			None => {
				if has_impl_all {
					unnamed_variant_type(&variant).expect("unsupported enum variant field")
				} else {
					continue;
				}
			}
		};

		let variant_name = &variant.ident;

		let gen = match variant.fields {
			Fields::Unnamed(_) => {
				quote! {
					impl From<#foreign_type> for #enum_name {
						fn from(v: #foreign_type) -> Self {
							Self::#variant_name(v)
						}
					}
				}
			}
			Fields::Unit => {
				quote! {
					impl From<#foreign_type> for #enum_name {
						fn from(_: #foreign_type) -> Self {
							Self::#variant_name
						}
					}
				}
			}
			_ => panic!("Named variants not supported"),
		};

		impls.push(gen);
	}

	impls.into_iter().map(TokenStream::from).collect()
}

fn unnamed_variant_type(variant: &Variant) -> Option<Type> {
	match variant.fields {
		Fields::Unnamed(ref fields) => fields.unnamed.first().map(|field| field.ty.clone()),
		_ => panic!("unsupported enum variant field"),
	}
}
