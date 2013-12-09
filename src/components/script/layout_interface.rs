/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! The high-level interface from script to layout. Using this abstract interface helps reduce
/// coupling between these two components, and enables the DOM to be placed in a separate crate
/// from layout.

use dom::node::{AbstractNode, LayoutDataRef, LayoutView, ScriptView};

use extra::url::Url;
use geom::point::Point2D;
use geom::rect::Rect;
use geom::size::Size2D;
use script_task::{ScriptChan};
use servo_util::geometry::Au;
use std::comm::{Chan, SharedChan};
use std::cmp;
use style::Stylesheet;

/// Asynchronous messages that script can send to layout.
///
/// FIXME(pcwalton): I think this should probably be merged with `LayoutQuery` below.
pub enum Msg {
    /// Adds the given stylesheet to the document.
    AddStylesheetMsg(Stylesheet),

    /// Requests a reflow.
    ReflowMsg(~Reflow),

    /// Performs a synchronous layout request.
    ///
    /// FIXME(pcwalton): As noted below, this isn't very type safe.
    QueryMsg(LayoutQuery),

    /// Destroys layout data associated with a DOM node.
    ///
    /// TODO(pcwalton): Maybe think about batching to avoid message traffic.
    ReapLayoutDataMsg(LayoutDataRef),

    /// Requests that the layout task enter a quiescent state in which no more messages are
    /// accepted except `ExitMsg`. A response message will be sent on the supplied channel when
    /// this happens.
    PrepareToExitMsg(Chan<()>),

    /// Requests that the layout task immediately shut down. There must be no more nodes left after
    /// this, or layout will crash.
    ExitNowMsg,
}

/// Synchronous messages that script can send to layout.
pub enum LayoutQuery {
    /// Requests the dimensions of the content box, as in the `getBoundingClientRect()` call.
    ContentBoxQuery(AbstractNode<ScriptView>, Chan<ContentBoxResponse>),
    /// Requests the dimensions of all the content boxes, as in the `getClientRects()` call.
    ContentBoxesQuery(AbstractNode<ScriptView>, Chan<ContentBoxesResponse>),
    /// Requests the node containing the point of interest
    HitTestQuery(AbstractNode<ScriptView>, Point2D<f32>, Chan<Result<HitTestResponse, ()>>),
}

pub struct ContentBoxResponse(Rect<Au>);
pub struct ContentBoxesResponse(~[Rect<Au>]);
pub struct HitTestResponse(AbstractNode<LayoutView>);

/// Determines which part of the 
#[deriving(Eq, Ord)]
pub enum DocumentDamageLevel {
    /// Reflow, but do not perform CSS selector matching.
    ReflowDocumentDamage,
    /// Perform CSS selector matching and reflow.
    MatchSelectorsDocumentDamage,
}

impl DocumentDamageLevel {
    /// Sets this damage to the maximum of this damage and the given damage.
    pub fn add(&mut self, new_damage: DocumentDamageLevel) {
        *self = cmp::max(*self, new_damage);
    }
}

/// What parts of the document have changed, as far as the script task can tell.
///
/// Note that this is fairly coarse-grained and is separate from layout's notion of the document
pub struct DocumentDamage {
    /// The topmost node in the tree that has changed.
    root: AbstractNode<ScriptView>,
    /// The amount of damage that occurred.
    level: DocumentDamageLevel,
}

/// Why we're doing reflow.
#[deriving(Eq)]
pub enum ReflowGoal {
    /// We're reflowing in order to send a display list to the screen.
    ReflowForDisplay,
    /// We're reflowing in order to satisfy a script query. No display list will be created.
    ReflowForScriptQuery,
}

/// Information needed for a reflow.
pub struct Reflow {
    /// The document node.
    document_root: AbstractNode<ScriptView>,
    /// The style changes that need to be done.
    damage: DocumentDamage,
    /// The goal of reflow: either to render to the screen or to flush layout info for script.
    goal: ReflowGoal,
    /// The URL of the page.
    url: Url,
    /// The channel through which messages can be sent back to the script task.
    script_chan: ScriptChan,
    /// The current window size.
    window_size: Size2D<uint>,
    /// The channel that we send a notification to.
    script_join_chan: Chan<()>,
    /// Unique identifier
    id: uint
}

/// Encapsulates a channel to the layout task.
#[deriving(Clone)]
pub struct LayoutChan(SharedChan<Msg>);
impl LayoutChan {
    pub fn new(chan: Chan<Msg>) -> LayoutChan {
        LayoutChan(SharedChan::new(chan))
    }
}

#[test]
fn test_add_damage() {
    fn assert_add(mut a: DocumentDamageLevel, b: DocumentDamageLevel,
                  result: DocumentDamageLevel) {
        a.add(b);
        assert!(a == result);
    }

    assert_add(ReflowDocumentDamage, ReflowDocumentDamage, ReflowDocumentDamage);
    assert_add(ReflowDocumentDamage, MatchSelectorsDocumentDamage, MatchSelectorsDocumentDamage);
    assert_add(MatchSelectorsDocumentDamage, ReflowDocumentDamage, MatchSelectorsDocumentDamage);
}
