// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

pub use rustdoc_types::ItemKind;

#[derive(Debug, Serialize)]
pub struct ItemDoc {
    pub name: String,
    pub kind: ItemKind,
    pub docs: Option<String>,
    pub implemented_traits: Vec<String>,
    pub methods: Vec<MethodDoc>,
    pub fields: Vec<FieldDoc>,
    pub variants: Vec<VariantDoc>,
}

#[derive(Debug, Serialize)]
pub struct MethodDoc {
    pub name: String,
    pub docs: Option<String>,
    pub args: Vec<ArgDoc>,
    pub return_type: String,
}

#[derive(Debug, Serialize)]
pub struct ArgDoc {
    pub name: String,
    pub type_name: String,
    pub docs: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FieldDoc {
    pub name: String,
    pub type_name: String,
    pub docs: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VariantDoc {
    pub name: String,
    pub docs: Option<String>,
    pub fields: Vec<FieldDoc>,
}
