//  flotte-user-management server for managing users, roles and permissions
//  Copyright (C) 2020 trivernis
//  See LICENSE for more information

#![allow(dead_code)]

pub(crate) const NULL: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
pub(crate) const ERROR: [u8; 4] = [0x0F, 0x0F, 0x0F, 0x0F];
pub(crate) const INFO: [u8; 4] = [0x49, 0x4e, 0x46, 0x4f];
pub(crate) const VALIDATE_TOKEN: [u8; 4] = [0x56, 0x41, 0x4c, 0x49];
pub(crate) const GET_ROLES: [u8; 4] = [0x52, 0x4f, 0x4c, 0x45];
pub(crate) const GET_ROLE_PERMISSIONS: [u8; 4] = [0x50, 0x45, 0x52, 0x4d];
pub(crate) const CREATE_ROLE: [u8; 4] = [0x43, 0x52, 0x4f, 0x4c];
pub(crate) const CREATE_PERMISSION: [u8; 4] = [0x43, 0x50, 0x45, 0x52];
pub(crate) const GET_USER_ID: [u8; 4] = [0x55, 0x53, 0x45, 0x52];
