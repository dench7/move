// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// This file defines functions for generating JSON-ABI.

use serde::{Deserialize, Serialize};

use crate::{
    attributes::FunctionAttribute,
    events::EventSignature,
    solidity_ty::{SoliditySignature, SolidityType},
};

#[derive(Serialize, Deserialize)]
pub(crate) struct ABIJsonArg {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ABIJsonArg>>,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ABIJsonSignature {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub inputs: Vec<ABIJsonArg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<ABIJsonArg>>,
    #[serde(rename = "stateMutability", skip_serializing_if = "Option::is_none")]
    pub state_mutability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anonymous: Option<bool>,
}

impl ABIJsonArg {
    pub(crate) fn from_ty(ty: &SolidityType, name: String) -> Self {
        use SolidityType::*;
        let mut abi_ty_str = ty.to_string();
        let mut components = None;
        if let Struct(_, ty_tuples) = ty {
            let mut comps = vec![];
            abi_ty_str = "tuple".to_string();
            for (_, _, _, para_name, comp_ty) in ty_tuples {
                let t = Self::from_ty(comp_ty, para_name.clone());
                comps.push(t);
            }
            components = Some(comps);
        } else if matches!(ty, DynamicArray(_)) || matches!(ty, StaticArray(_, _)) {
            let mut array_vec = vec![];
            let base_ty = find_inner_ty_from_array(ty, &mut array_vec);
            if let Struct(_, ty_tuples) = base_ty {
                let mut comps = vec![];
                abi_ty_str = "tuple".to_string();
                for dimension in array_vec.into_iter().rev() {
                    let dim = if dimension > 0 {
                        dimension.to_string()
                    } else {
                        "".to_string()
                    };
                    abi_ty_str = format!("{}[{}]", abi_ty_str, dim);
                }
                for (_, _, _, para_name, comp_ty) in ty_tuples {
                    let t = Self::from_ty(&comp_ty, para_name.clone());
                    comps.push(t);
                }
                components = Some(comps);
            }
        }
        ABIJsonArg {
            ty: abi_ty_str,
            indexed: None,
            components,
            name,
        }
    }

    pub(crate) fn from_event_ty(ty: &SolidityType, indexed: bool, name: String) -> Self {
        let abi = Self::from_ty(ty, name);
        ABIJsonArg {
            ty: abi.ty,
            indexed: Some(indexed),
            components: abi.components,
            name: abi.name,
        }
    }
}

fn find_inner_ty_from_array(ty: &SolidityType, para: &mut Vec<usize>) -> SolidityType {
    use SolidityType::*;
    let mut ret_ty = ty.clone();
    if let DynamicArray(inner_ty) = ty {
        ret_ty = *inner_ty.clone();
        para.push(0);
    } else if let StaticArray(inner_ty, m) = ty {
        ret_ty = *inner_ty.clone();
        para.push(*m);
    }
    if ret_ty.is_array() {
        find_inner_ty_from_array(&ret_ty, para)
    } else {
        ret_ty
    }
}

impl ABIJsonSignature {
    pub(crate) fn from_solidity_sig(
        sig: &SoliditySignature,
        attr: Option<FunctionAttribute>,
        fun_typ: &str,
    ) -> Self {
        let name = sig.sig_name.clone();
        let mut inputs = vec![];
        let mut outputs = vec![];
        for (ty, para_name, _) in &sig.para_types {
            inputs.push(ABIJsonArg::from_ty(ty, para_name.clone()));
        }
        for (ty, _) in &sig.ret_types {
            outputs.push(ABIJsonArg::from_ty(ty, "".to_string()));
        }
        let state_mutability = (if let Some(FunctionAttribute::View) = attr {
            "view"
        } else if let Some(FunctionAttribute::Pure) = attr {
            "pure"
        } else if let Some(FunctionAttribute::Payable) = attr {
            "payable"
        } else {
            "nonpayable"
        })
        .to_string();
        let anonymous = None;
        ABIJsonSignature {
            name,
            ty: fun_typ.to_string(),
            inputs,
            outputs: Some(outputs),
            state_mutability: Some(state_mutability),
            anonymous,
        }
    }

    pub(crate) fn from_event_sig(sig: &EventSignature) -> Self {
        let name = sig.event_name.clone();
        let ty = "event".to_string();
        let mut inputs = vec![];
        for (_, ty, _, indexed_flag, ev_name) in &sig.para_types {
            inputs.push(ABIJsonArg::from_event_ty(
                ty,
                *indexed_flag,
                ev_name.clone(),
            ));
        }
        ABIJsonSignature {
            name,
            ty,
            inputs,
            outputs: None,
            state_mutability: None,
            anonymous: Some(false),
        }
    }
}
