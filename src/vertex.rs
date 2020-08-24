use ash::vk;

// TODO: Cleanup traits to use this
#[derive(Debug, Clone)]
pub struct VertexFormat {
    pub binding_description: &vk::VertexInputBindingDescription,
    pub attribute_description: &[vk::VertexInputAttributeDescription],
}

pub trait VertexDefinition {
    fn vertex_format() -> VertexFormat;
}

pub trait VertexSource {
    fn binding_description(&self) -> Vec<vk::VertexInputBindingDescription>;
    fn attribute_description(&self) -> Vec<vk::VertexInputAttributeDescription>;
}

impl<V: VertexDefinition> VertexSource for Vec<V> {
    fn binding_description(&self) -> Vec<vk::VertexInputBindingDescription> {
        V::binding_description()
    }

    fn attribute_description(&self) -> Vec<vk::VertexInputAttributeDescription> {
        V::attribute_description()
    }
}


pub const SIZE_TO_VK_FORMAT: [vk::Format; 5] = [vk::Format::UNDEFINED, vk::Format::R32_SFLOAT,
       vk::Format::R32G32_SFLOAT,
        vk::Format::R32G32B32_SFLOAT,
        vk::Format::R32G32B32A32_SFLOAT,];

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Data, DataStruct};

#[proc_macro_derive(Vertex)]
pub fn writable_template_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let input = syn::parse_macro_input!(input as DeriveInput);
  // TODO: Check that struct is packed/C/transparent (whichever is the right one)

  // get the name of the type we want to implement the trait for
  let struct_name = &input.ident;
  let members = if let Data::Struct(data) = input.data {
    match data.fields {
        Fields::Named(ref fields) => {
            fields.named.iter().enumerate().map(|(idx, f)| {
                let field_name = &f.ident;
                let field_ty = &f.ty;
                quote! {
                    vk::VertexInputAttributeDescription {
                        binding: 0,
                        location: #idx,
                        format: crate::vertex::size_to_vk_format(std::mem::size_of<#field_ty>()),
                        offset: memoffset::offset_of!(#struct_name, #field_name) as u32,
                    },
                }
            }).collect::<Vec<_>>()
        },
        _ => unimplemented!(),
    }
  } else {
    unimplemented!()
  };

    // TODO: Put in separate struct?

  let binding_description_varname = quote::format_ident!("_trekanten_vertex_format_input_description_{}", struct_name);
  let attribute_description_varname = quote::format_ident!("_trekanten_vertex_format_attribute_binding_{}", struct_name);
  
  let n_members = members.len();
  assert_eq!(n_members > 0);

  let expanded = quote! {
    const #binding_description_varname: [ash::vk::VertexInputBindingDescription; 1] = [ash::vk::VertexInputBindingDescription {
        binding: 0,
        stride: std::mem::size_of::<#struct_name>() as u32,
        input_rat: ash::vk::VertexInputRate::VERTEX,
    }];

    const #attribute_description_varname: [ash::vk::VertexAttributeBindingDescription; #n_members] = [#(#recurse)*];

    impl crate::vertex::VertexDefinition for #struct_name {
      fn vertex_format(&self) -> crate::vertex::VertexFormat {
          crate::vertex::VertexFormat {
            binding_description: &#binding_description_varname,
            attribute_description: &#attribute_description_varname,
          }
      }
    }
  };

  proc_macro::TokenStream::from(expanded)
}

