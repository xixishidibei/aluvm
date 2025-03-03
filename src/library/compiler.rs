// Reference rust implementation of AluVM (arithmetic logic unit virtual machine).
// To find more on AluVM please check <https://aluvm.org>
//
// SPDX-License-Identifier: Apache-2.0
//
// Designed in 2021-2025 by Dr Maxim Orlovsky <orlovsky@ubideco.org>
// Written in 2021-2025 by Dr Maxim Orlovsky <orlovsky@ubideco.org>
//
// Copyright (C) 2021-2024 LNP/BP Standards Association, Switzerland.
// Copyright (C) 2024-2025 Laboratories for Ubiquitous Deterministic Computing (UBIDECO),
//                         Institute for Distributed and Cognitive Systems (InDCS), Switzerland.
// Copyright (C) 2021-2025 Dr Maxim Orlovsky.
// All rights under the above copyrights are reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License. You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

use alloc::vec::Vec;

use crate::isa::Instruction;
use crate::library::assembler::AssemblerError;
use crate::{Lib, LibId, LibSite};

#[derive(Clone, Eq, PartialEq, Hash, Debug, Display, Error, From)]
#[display(doc_comments)]
pub enum CompilerError<Isa: Instruction<LibId>> {
    #[from]
    #[display(inner)]
    Assemble(AssemblerError),

    /// instruction number {1} `{0}` (offset {2:#x}) references goto target absent in the code. Use
    /// `nop` instruction to mark the goto target.
    ///
    /// The known goto target offsets are: {3:#x?}
    InvalidRef(Isa, usize, u16, Vec<u16>),
}

pub struct CompiledLib {
    id: LibId,
    lib: Lib,
    routines: Vec<u16>,
}

impl CompiledLib {
    /// Compiles library from the provided instructions by resolving local call pointers first, and
    /// then assembling it into a bytecode by calling [`Self::assemble`].
    pub fn compile<Isa>(mut code: impl AsMut<[Isa]>) -> Result<Self, CompilerError<Isa>>
    where Isa: Instruction<LibId> {
        let code = code.as_mut();
        let mut routines = vec![];
        let mut cursor = 0u16;
        for instr in &*code {
            if instr.is_local_goto_target() {
                routines.push(cursor);
            }
            cursor += instr.code_byte_len();
        }
        let mut cursor = 0u16;
        for (no, instr) in code.iter_mut().enumerate() {
            let Some(goto_pos) = instr.local_goto_pos() else {
                cursor += instr.code_byte_len();
                continue;
            };
            let Some(pos) = routines.get(*goto_pos as usize) else {
                return Err(CompilerError::InvalidRef(instr.clone(), no, cursor, routines));
            };
            *goto_pos = *pos;
            cursor += instr.code_byte_len();
        }
        let lib = Lib::assemble(code)?;
        let id = lib.lib_id();
        Ok(Self { id, lib, routines })
    }

    pub fn routines_count(&self) -> usize { self.routines.len() }

    /// Returns code offset for the entry point of a given routine.
    ///
    /// # Panics
    ///
    /// Panics if the routine with the given number is not defined
    pub fn routine(&self, no: u16) -> LibSite {
        let pos = self.routines[no as usize];
        LibSite::new(self.id, pos)
    }

    pub fn as_lib(&self) -> &Lib { &self.lib }

    pub fn into_lib(self) -> Lib { self.lib }
}
