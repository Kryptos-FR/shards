/* SPDX-License-Identifier: BSD-3-Clause */
/* Copyright © 2021 Fragcolor Pte. Ltd. */

use crate::shard::Shard;
use crate::shardsc::SHTypeInfo_Details_Object;
use crate::shardsc::SHType_Bool;
use crate::shardsc::SHType_Bytes;
use crate::shardsc::SHType_Int;
use crate::shardsc::SHType_None;
use crate::shardsc::SHType_Seq;
use crate::shardsc::SHType_String;
use crate::core::do_blocking;
use crate::core::log;
use crate::core::registerShard;
use crate::types::common_type;
use crate::types::ClonedVar;
use crate::types::Context;
use crate::types::InstanceData;
use crate::types::OptionalString;
use crate::types::ParamVar;
use crate::types::Parameters;
use crate::types::Seq;
use crate::types::Table;
use crate::types::Type;
use crate::types::ANYS_TYPES;
use crate::types::BYTES_TYPES;
use crate::types::ENUMS_TYPE;
use crate::types::FRAG_CC;
use crate::types::STRINGS_TYPES;
use crate::types::STRING_TYPES;
use crate::CString;
use crate::Types;
use crate::Var;
use core::time::Duration;
use parity_scale_codec::{Compact, Decode, Encode, HasCompact};
use sp_core::crypto::{AccountId32, Pair, Ss58Codec};
use sp_core::storage::StorageKey;
use sp_core::{blake2_128, ecdsa, ed25519, sr25519, twox_128};
use sp_runtime::generic::Era;
use sp_runtime::{MultiAddress, MultiSignature, MultiSigner};
use std::convert::{TryFrom, TryInto};
use std::ffi::CStr;
use std::rc::Rc;
use std::str::FromStr;

lazy_static! {
  static ref ID_PARAMETERS: Parameters = vec![(
    cstr!("ECDSA"),
    shccstr!("If the input public key is an ECDSA and not the default Sr25519/Ed25519."),
    vec![common_type::bool]
  )
    .into(),
  (
    cstr!("Version"),
    shccstr!("The substrate version prefix."),
    vec![common_type::int]
  )
    .into()];
  static ref STORAGE_PARAMETERS: Parameters = vec![(
      cstr!("PreHashed"),
      shccstr!("If the keys are already hashed."),
      vec![common_type::bool]
    )
    .into()];
  static ref STRINGS_OR_NONE: Vec<Type> = vec![common_type::string, common_type::none];
  static ref STRINGS_OR_NONE_TYPE: Type = Type::seq(&STRINGS_OR_NONE);
  static ref ENCODE_PARAMETERS: Parameters = vec![(
    cstr!("Hints"),
    shccstr!("The hints of types to encode, either \"i8\"/\"u8\", \"i16\"/\"u16\" etc... for int types, \"c\" for Compact int, \"a\" for AccountId or nil for other types."),
    vec![*STRINGS_OR_NONE_TYPE]
  )
    .into()];
  static ref DECODE_PARAMETERS: Parameters = vec![(
    cstr!("Types"),
    shccstr!("The list of types expected to decode."),
    vec![*ENUMS_TYPE]
  )
    .into(),
  (
    cstr!("Hints"),
    shccstr!("The hints of the types to decode, either \"i8\"/\"u8\", \"i16\"/\"u16\" etc... for int types, \"c\" for Compact int, \"a\" for AccountId or nil for other types."),
    vec![*STRINGS_OR_NONE_TYPE]
  )
    .into(),
  (
    cstr!("Version"),
    shccstr!("The substrate version prefix."),
    vec![common_type::int]
  )
    .into()];
  static ref METADATA_TYPE: Type = {
    let mut t = common_type::object;
    t.details.object = SHTypeInfo_Details_Object {
      vendorId: FRAG_CC, // 'frag'
      typeId: 0x7375624D, // 'subM'
    };
    t
  };
  static ref METADATA_TYPES: Vec<Type> = vec![*METADATA_TYPE];
}

fn get_key<T: Pair>(input: Var) -> Result<T, &'static str> {
  let key: Result<&[u8], &str> = input.as_ref().try_into();
  if let Ok(key) = key {
    T::from_seed_slice(key).map_err(|e| {
      shlog!("{:?}", e);
      "Failed to parse secret key"
    })
  } else {
    let key: Result<&str, &str> = input.as_ref().try_into();
    if let Ok(key) = key {
      T::from_string(key, None).map_err(|e| {
        shlog!("{:?}", e);
        "Failed to parse secret key"
      })
    } else {
      Err("Invalid key value")
    }
  }
}

struct AccountId {
  output: ClonedVar,
  is_ecdsa: bool,
  version: i64,
}

impl Default for AccountId {
  fn default() -> Self {
    Self {
      output: ClonedVar::default(),
      is_ecdsa: false,
      version: 42, // substrate default
    }
  }
}

impl Shard for AccountId {
  fn registerName() -> &'static str {
    cstr!("Substrate.AccountId")
  }

  fn hash() -> u32 {
    compile_time_crc32::crc32!("Substrate.AccountId-rust-0x20200101")
  }

  fn name(&mut self) -> &str {
    "Substrate.AccountId"
  }

  fn help(&mut self) -> OptionalString {
    OptionalString(shccstr!("Returns the account id of the input public key."))
  }

  fn inputTypes(&mut self) -> &Vec<Type> {
    &BYTES_TYPES
  }

  fn inputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!(
      "The public key bytes, must be a valid Sr25519 or Ed25519 public key."
    ))
  }

  fn outputTypes(&mut self) -> &Vec<Type> {
    &STRING_TYPES
  }

  fn outputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("The Substrate account id in SS58 format."))
  }

  fn parameters(&mut self) -> Option<&Parameters> {
    Some(&ID_PARAMETERS)
  }

  fn setParam(&mut self, index: i32, value: &Var) {
    match index {
      0 => self.is_ecdsa = value.try_into().unwrap(),
      1 => self.version = value.try_into().unwrap(),
      _ => unreachable!(),
    }
  }

  fn getParam(&mut self, index: i32) -> Var {
    match index {
      0 => self.is_ecdsa.into(),
      1 => self.version.into(),
      _ => unreachable!(),
    }
  }

  fn activate(&mut self, _: &Context, input: &Var) -> Result<Var, &str> {
    let bytes: &[u8] = input.as_ref().try_into()?;
    let prefix: u16 = self
      .version
      .try_into()
      .map_err(|_| "Invalid version, out of u16 range")?;
    let id: String = if self.is_ecdsa {
      let raw: [u8; 33] = bytes.try_into().map_err(|_| "Invalid key length")?;
      let key = ecdsa::Public::from_raw(raw);
      key.to_ss58check_with_version(prefix.into())
    } else {
      let raw: [u8; 32] = bytes.try_into().map_err(|_| "Invalid key length")?;
      let key = sr25519::Public::from_raw(raw);
      key.to_ss58check_with_version(prefix.into())
    };
    self.output = id.into();
    Ok(self.output.0)
  }
}

#[derive(Default)]
struct SHStorageKey {
  output: ClonedVar,
  v: Vec<u8>,
}

impl Shard for SHStorageKey {
  fn registerName() -> &'static str {
    cstr!("Substrate.StorageKey")
  }

  fn hash() -> u32 {
    compile_time_crc32::crc32!("Substrate.StorageKey-rust-0x20200101")
  }

  fn name(&mut self) -> &str {
    "Substrate.StorageKey"
  }

  fn help(&mut self) -> OptionalString {
    OptionalString(shccstr!(
      "Returns the encoded storage key corresponding to the input strings."
    ))
  }

  fn inputTypes(&mut self) -> &Vec<Type> {
    &STRINGS_TYPES
  }

  fn inputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!(
      "A sequence of two strings representing the key we want to encode."
    ))
  }

  fn outputTypes(&mut self) -> &Vec<Type> {
    &BYTES_TYPES
  }

  fn outputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("The encoded storage key."))
  }

  fn activate(&mut self, _: &Context, input: &Var) -> Result<Var, &str> {
    let strings: Seq = input.try_into()?;
    if strings.len() != 2 {
      return Err("Invalid input length");
    }
    self.v.clear();
    for string in strings {
      let string: &str = string.as_ref().try_into()?;
      let hash = twox_128(string.as_bytes());
      self.v.extend(&hash[..]);
    }
    self.output = self.v.as_slice().into();
    Ok(self.output.0)
  }
}

#[derive(Default)]
struct SHStorageMap {
  output: ClonedVar,
  v: Vec<u8>,
  pre_hashed: bool,
}

impl Shard for SHStorageMap {
  fn registerName() -> &'static str {
    cstr!("Substrate.StorageMap")
  }

  fn hash() -> u32 {
    compile_time_crc32::crc32!("Substrate.StorageMap-rusts-0x20200101")
  }

  fn name(&mut self) -> &str {
    "Substrate.StorageMap"
  }

  fn help(&mut self) -> OptionalString {
    OptionalString(shccstr!(
      "Returns the encoded storage map corresponding to the input strings."
    ))
  }

  fn inputTypes(&mut self) -> &Vec<Type> {
    &STRINGS_TYPES
  }

  fn inputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!(
      "A sequence of three strings representing the map we want to encode, the last one being SCALE encoded data of the key."
    ))
  }

  fn outputTypes(&mut self) -> &Vec<Type> {
    &BYTES_TYPES
  }

  fn outputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("The encoded storage map."))
  }

  fn parameters(&mut self) -> Option<&Parameters> {
    Some(&STORAGE_PARAMETERS)
  }

  fn setParam(&mut self, index: i32, value: &Var) {
    match index {
      0 => self.pre_hashed = value.try_into().unwrap(),
      _ => unreachable!(),
    }
  }

  fn getParam(&mut self, index: i32) -> Var {
    match index {
      0 => self.pre_hashed.into(),
      _ => unreachable!(),
    }
  }

  fn activate(&mut self, _: &Context, input: &Var) -> Result<Var, &str> {
    let strings: Seq = input.try_into()?;

    if strings.len() < 3 {
      return Err("Invalid input length");
    }

    self.v.clear();

    // first two strings are the map
    for i in 0..2 {
      let string: &str = strings[i].as_ref().try_into()?;
      let hash = twox_128(string.as_bytes());
      self.v.extend(&hash[..]);
    }

    // third is the key
    let string: &str = strings[2].as_ref().try_into()?;
    let bytes =
      hex::decode(string.trim_start_matches("0x").trim_start_matches("0X")).map_err(|e| {
        shlog!("{}", e);
        "Failed to parse input hex string"
      })?;
    if self.pre_hashed {
      self.v.extend(&bytes[..]);
    } else {
      // default to blake2_128 concat
      let hash = blake2_128(&bytes);
      self.v.extend(&hash[..]);
      self.v.extend(bytes);
    }

    // double map key
    if strings.len() == 4 {
      let string: &str = strings[3].as_ref().try_into()?;
      let bytes =
        hex::decode(string.trim_start_matches("0x").trim_start_matches("0X")).map_err(|e| {
          shlog!("{}", e);
          "Failed to parse input hex string"
        })?;
      if self.pre_hashed {
        self.v.extend(&bytes[..]);
      } else {
        // default to blake2_128 concat
        let hash = blake2_128(&bytes);
        self.v.extend(&hash[..]);
        self.v.extend(bytes);
      }
    }

    self.output = self.v.as_slice().into();
    Ok(self.output.0)
  }
}

#[derive(Default)]
struct SHEncode {
  output: ClonedVar,
  hints: ClonedVar,
  v: Vec<u8>,
}

fn encode_var(value: Var, hint: Var, dest: &mut Vec<u8>) -> Result<(), &'static str> {
  match value.valueType {
    SHType_None => {
      dest.push(0);
      Ok(())
    }
    SHType_String => {
      let hint: Result<&str, &str> = hint.as_ref().try_into();
      let account = if let Ok(hint) = hint {
        matches!(hint, "a")
      } else {
        false
      };
      let string: &str = value.as_ref().try_into()?;
      if account {
        let id = AccountId32::from_str(string).map_err(|_| "Invalid account id")?;
        id.encode_to(dest);
        Ok(())
      } else {
        string.encode_to(dest);
        Ok(())
      }
    }
    SHType_Int => {
      let hint: &str = hint.as_ref().try_into()?;
      let int: i64 = value.as_ref().try_into()?;
      match hint {
        "u8" => {
          u8::encode_to(&int.try_into().map_err(|_| "Invalid u8 value")?, dest);
          Ok(())
        }
        "u16" => {
          u16::encode_to(&int.try_into().map_err(|_| "Invalid u16 value")?, dest);
          Ok(())
        }
        "u32" => {
          u32::encode_to(&int.try_into().map_err(|_| "Invalid u32 value")?, dest);
          Ok(())
        }
        "u64" => {
          u64::encode_to(&int.try_into().map_err(|_| "Invalid u64 value")?, dest);
          Ok(())
        }
        "u128" => {
          u128::encode_to(&int.try_into().map_err(|_| "Invalid u128 value")?, dest);
          Ok(())
        }
        "i8" => {
          i8::encode_to(&int.try_into().map_err(|_| "Invalid i8 value")?, dest);
          Ok(())
        }
        "i16" => {
          i16::encode_to(&int.try_into().map_err(|_| "Invalid i16 value")?, dest);
          Ok(())
        }
        "i32" => {
          i32::encode_to(&int.try_into().map_err(|_| "Invalid i32 value")?, dest);
          Ok(())
        }
        "i64" => {
          i64::encode_to(&int, dest);
          Ok(())
        }
        "i128" => {
          i128::encode_to(&int.try_into().map_err(|_| "Invalid i128 value")?, dest);
          Ok(())
        }
        "c" => {
          let u64int: u64 = int.try_into().map_err(|_| "Invalid u64 value")?;
          Compact(u64int).encode_to(dest);
          Ok(())
        }
        _ => Err("Invalid hint"),
      }
    }
    SHType_Bytes => {
      let bytes: &[u8] = value.as_ref().try_into()?;
      bytes.encode_to(dest);
      Ok(())
    }
    SHType_Bool => {
      let bool: bool = value.as_ref().try_into()?;
      bool.encode_to(dest);
      Ok(())
    }
    _ => Err("Invalid input value type"),
  }
}

impl Shard for SHEncode {
  fn registerName() -> &'static str {
    cstr!("Substrate.Encode")
  }

  fn hash() -> u32 {
    compile_time_crc32::crc32!("Substrate.Encode-rust-0x20200101")
  }

  fn name(&mut self) -> &str {
    "Substrate.Encode"
  }

  fn help(&mut self) -> OptionalString {
    OptionalString(shccstr!("Encode values into a byte array in SCALE format"))
  }

  fn inputTypes(&mut self) -> &Vec<Type> {
    &ANYS_TYPES
  }

  fn inputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("A sequence of values to SCALE encode."))
  }

  fn outputTypes(&mut self) -> &Vec<Type> {
    &BYTES_TYPES
  }

  fn outputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("The encoded SCALE bytes."))
  }

  fn parameters(&mut self) -> Option<&Parameters> {
    Some(&ENCODE_PARAMETERS)
  }

  fn setParam(&mut self, index: i32, value: &Var) {
    match index {
      0 => self.hints = value.into(),
      _ => unreachable!(),
    }
  }

  fn getParam(&mut self, index: i32) -> Var {
    match index {
      0 => self.hints.0,
      _ => unreachable!(),
    }
  }

  fn activate(&mut self, _: &Context, input: &Var) -> Result<Var, &str> {
    let values: Seq = input.try_into()?;
    let hints: Seq = self.hints.0.try_into()?;

    if values.len() != hints.len() {
      return Err("Invalid input length");
    }

    self.v.clear();
    for (value, hint) in values.iter().zip(hints.iter()) {
      encode_var(value, hint, &mut self.v)?;
    }

    self.output = self.v.as_slice().into();
    Ok(self.output.0)
  }
}

struct SHDecode {
  output: Seq,
  hints: ClonedVar,
  types: ClonedVar,
  version: i64,
}

impl Default for SHDecode {
  fn default() -> Self {
    Self {
      output: Seq::new(),
      hints: ClonedVar(Var::default()),
      types: ClonedVar(Var::default()),
      version: 42, // substrate default
    }
  }
}

impl Shard for SHDecode {
  fn registerName() -> &'static str {
    cstr!("Substrate.Decode")
  }

  fn hash() -> u32 {
    compile_time_crc32::crc32!("Substrate.Decode-rust-0x20200101")
  }

  fn name(&mut self) -> &str {
    "Substrate.Decode"
  }

  fn help(&mut self) -> OptionalString {
    OptionalString(shccstr!("Decode SCALE encoded bytes into values"))
  }

  fn inputTypes(&mut self) -> &Vec<Type> {
    &BYTES_TYPES
  }

  fn inputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("The encoded SCALE bytes."))
  }

  fn outputTypes(&mut self) -> &Vec<Type> {
    &ANYS_TYPES
  }

  fn outputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("The decoded values."))
  }

  fn parameters(&mut self) -> Option<&Parameters> {
    Some(&DECODE_PARAMETERS)
  }

  fn setParam(&mut self, index: i32, value: &Var) {
    match index {
      0 => self.types = value.into(),
      1 => self.hints = value.into(),
      2 => self.version = value.try_into().expect("A valid integer var"),
      _ => unreachable!(),
    }
  }

  fn getParam(&mut self, index: i32) -> Var {
    match index {
      0 => self.types.0,
      1 => self.hints.0,
      2 => self.version.into(),
      _ => unreachable!(),
    }
  }

  fn activate(&mut self, _: &Context, input: &Var) -> Result<Var, &str> {
    let bytes: &[u8] = input.as_ref().try_into()?;
    let types: Seq = self.types.0.try_into()?;
    let hints: Seq = self.hints.0.try_into()?;
    let mut offset = 0;
    self.output.clear();
    for (t, h) in types.iter().zip(hints.iter()) {
      let t = t.enum_value()?;
      match t {
        0 => {
          // None
          offset += 1;
          self.output.push(().into());
        }
        2 => {
          // Bool
          let mut bytes = &bytes[offset..];
          let value = bool::decode(&mut bytes).map_err(|_| "Invalid bool")?;
          offset += 1;
          self.output.push(value.into());
        }
        3 => {
          // Int
          let hint: &str = h.as_ref().try_into()?;
          let mut bytes = &bytes[offset..];
          let value = match hint {
            "u8" => {
              let value = u8::decode(&mut bytes).map_err(|_| "Invalid u8")?;
              offset += value.encoded_size();
              Ok(value.try_into()?)
            }
            "u16" => {
              let value = u16::decode(&mut bytes).map_err(|_| "Invalid u16")?;
              offset += value.encoded_size();
              Ok(value.try_into()?)
            }
            "u32" => {
              let value = u32::decode(&mut bytes).map_err(|_| "Invalid u32")?;
              offset += value.encoded_size();
              Ok(value.try_into()?)
            }
            "u64" => {
              let value = u64::decode(&mut bytes).map_err(|_| "Invalid u64")?;
              offset += value.encoded_size();
              Ok(value.try_into()?)
            }
            "u128" => {
              let value = u128::decode(&mut bytes).map_err(|_| "Invalid u128")?;
              offset += value.encoded_size();
              Ok(value.try_into()?)
            }
            "i8" => {
              let value = i8::decode(&mut bytes).map_err(|_| "Invalid i8")? as i64;
              offset += value.encoded_size();
              Ok(value.into())
            }
            "i16" => {
              let value = i16::decode(&mut bytes).map_err(|_| "Invalid i16")? as i64;
              offset += value.encoded_size();
              Ok(value.into())
            }
            "i32" => {
              let value = i32::decode(&mut bytes).map_err(|_| "Invalid i32")? as i64;
              offset += value.encoded_size();
              Ok(value.into())
            }
            "i64" => {
              let value = i64::decode(&mut bytes).map_err(|_| "Invalid i64")?;
              offset += value.encoded_size();
              Ok(value.into())
            }
            "i128" => {
              let value = i128::decode(&mut bytes).map_err(|_| "Invalid i128")?;
              offset += value.encoded_size();
              Ok(value.try_into()?)
            }
            "c" => {
              let value = Compact::<u64>::decode(&mut bytes).map_err(|_| "Invalid Compact int")?;
              offset += value.encoded_size();
              Ok(value.0.try_into()?)
            }
            _ => Err("Invalid hint"),
          }?;
          self.output.push(value);
        }
        16 => {
          // Bytes
          let mut bytes = &bytes[offset..];
          let value = Vec::<u8>::decode(&mut bytes).map_err(|_| "Invalid bytes")?;
          offset += value.encoded_size();
          self.output.push(value.as_slice().into());
        }
        17 => {
          // String
          let hint: Result<&str, &str> = h.as_ref().try_into();
          let account = if let Ok(hint) = hint {
            matches!(hint, "a")
          } else {
            false
          };
          let mut bytes = &bytes[offset..];
          if account {
            let prefix: u16 = self
              .version
              .try_into()
              .map_err(|_| "Invalid version, out of u16 range")?;
            let value = AccountId32::decode(&mut bytes)
              .map_err(|_| "Invalid account")?
              .to_ss58check_with_version(prefix.into());
            offset += value.encoded_size();
            self.output.push(value.as_str().into());
          } else {
            let value = String::decode(&mut bytes).map_err(|_| "Invalid string")?;
            offset += value.encoded_size();
            self.output.push(value.as_str().into());
          }
        }
        _ => return Err("Invalid value type"),
      }
    }
    Ok(self.output.as_ref().into())
  }
}

pub fn registerShards() {
  registerShard::<AccountId>();
  registerShard::<SHStorageKey>();
  registerShard::<SHStorageMap>();
  registerShard::<SHEncode>();
  registerShard::<SHDecode>();
}