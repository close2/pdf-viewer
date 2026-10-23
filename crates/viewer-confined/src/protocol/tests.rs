//! The wire's tests, one file per concern.

use pdf_model::action::DataFormat;
use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
use pdf_model::restriction::Operation;
use pdf_model::submission::{Format, Method, Submission};
use pdf_model::view::{Entered, Markup};
use pdf_render::{Point, Raster, RasterFormat, Rect, Size};
use pdf_sandbox::lockdown::Confinement;
use pdf_syntax::{Name, ObjectId};
use viewer_core::{
    Answer, Answered, AttachHome, Command, DocumentId, Edit, Event, Extraction, Find,
    FindDirection, FocusMove, Found, PageTarget, PointerAction, PresentationMode, Purpose, Query,
    RestrictionLevel, RestrictionOverride, RestrictionPolicy, RestrictionScope, Selection,
    TrustPolicy, Zoom,
};

use pdf_signature::verdict::Acceptance;

use crate::Reply;

use super::*;
use super::{
    answer::*, command::*, error::*, event::*, frame::*, marks::*, payload::*, query::*, wire::*,
};

mod commands;
mod events_queries;
mod frames;
mod panel_answers;
mod refusals;
