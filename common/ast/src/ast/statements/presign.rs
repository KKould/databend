// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::Display;
use std::fmt::Formatter;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresignAction {
    Download,
    Upload,
}

impl Default for PresignAction {
    fn default() -> Self {
        Self::Download
    }
}

impl Display for PresignAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PresignAction::Download => write!(f, "DOWNLOAD"),
            PresignAction::Upload => write!(f, "UPLOAD"),
        }
    }
}

/// TODO: we can support uri location in the future.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresignLocation {
    StageLocation {
        /// The name of the stage.
        name: String,
        path: String,
    },
}

impl Display for PresignLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PresignLocation::StageLocation { name, path } => {
                write!(f, "@{name}{path}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresignStmt {
    pub action: PresignAction,
    pub location: PresignLocation,
    pub expire: Duration,
}

impl Display for PresignStmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PRESIGN {} EXPIRE = {}",
            self.location,
            self.expire.as_secs()
        )
    }
}
