/* SPDX-License-Identifier: BSD-3-Clause */
/* Copyright © 2022 Fragcolor Pte. Ltd. */

use super::Label;
use crate::shard::Shard;
use crate::shards::gui::util;
use crate::shards::gui::BOOL_OR_NONE_SLICE;
use crate::shards::gui::PARENTS_UI_NAME;
use crate::types::Context;
use crate::types::ExposedTypes;
use crate::types::OptionalString;
use crate::types::ParamVar;
use crate::types::Parameters;
use crate::types::Types;
use crate::types::Var;
use crate::types::STRING_TYPES;

lazy_static! {
  static ref LABEL_PARAMETERS: Parameters = vec![(
    cstr!("Wrap"),
    cstr!("Wrap the text depending on the layout."),
    BOOL_OR_NONE_SLICE,
  )
    .into(),];
}

impl Default for Label {
  fn default() -> Self {
    let mut parents = ParamVar::default();
    parents.set_name(PARENTS_UI_NAME);
    Self {
      parents,
      requiring: Vec::new(),
      wrap: ParamVar::default(),
    }
  }
}

impl Shard for Label {
  fn registerName() -> &'static str
  where
    Self: Sized,
  {
    cstr!("UI.Label")
  }

  fn hash() -> u32
  where
    Self: Sized,
  {
    compile_time_crc32::crc32!("UI.Label-rust-0x20200101")
  }

  fn name(&mut self) -> &str {
    "UI.Label"
  }

  fn help(&mut self) -> OptionalString {
    OptionalString(shccstr!("Static text."))
  }

  fn inputTypes(&mut self) -> &Types {
    &STRING_TYPES
  }

  fn inputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("The text to display."))
  }

  fn outputTypes(&mut self) -> &Types {
    &STRING_TYPES
  }

  fn outputHelp(&mut self) -> OptionalString {
    OptionalString(shccstr!("The output of this shard will be its input."))
  }

  fn parameters(&mut self) -> Option<&Parameters> {
    Some(&LABEL_PARAMETERS)
  }

  fn setParam(&mut self, index: i32, value: &Var) -> Result<(), &str> {
    match index {
      0 => Ok(self.wrap.set_param(value)),
      _ => Err("Invalid parameter index"),
    }
  }

  fn getParam(&mut self, index: i32) -> Var {
    match index {
      0 => self.wrap.get_param(),
      _ => Var::default(),
    }
  }

  fn requiredVariables(&mut self) -> Option<&ExposedTypes> {
    self.requiring.clear();

    // Add UI.Parents to the list of required variables
    util::require_parents(&mut self.requiring, &self.parents);

    Some(&self.requiring)
  }

  fn warmup(&mut self, context: &Context) -> Result<(), &str> {
    self.parents.warmup(context);

    self.wrap.warmup(context);

    Ok(())
  }

  fn cleanup(&mut self) -> Result<(), &str> {
    self.parents.cleanup();

    self.wrap.cleanup();

    Ok(())
  }

  fn activate(&mut self, _context: &Context, input: &Var) -> Result<Var, &str> {
    if let Some(ui) = util::get_current_parent(*self.parents.get())? {
      let text: &str = input.try_into()?;
      let mut label = egui::Label::new(text);

      let wrap = self.wrap.get();
      if !wrap.is_none() {
        let wrap: bool = wrap.try_into()?;
        label = label.wrap(wrap);
      }

      ui.add(label);

      Ok(*input)
    } else {
      Err("No UI parent")
    }
  }
}
