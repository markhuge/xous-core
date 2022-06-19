use crate::*;
use gam::{UxRegistration, GlyphStyle};
use graphics_server::{Gid, Point, Rectangle, DrawStyle, PixelColor, TextView};
use xous::MessageEnvelope;
use std::fmt::Write;
use pddb::Pddb;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::io::{Read, Write as FsWrite};

/// Display list for items. "name" is the key by which the list is sorted.
/// "extra" is more information about the item, which should not be part of the sort.
struct ListItem {
    name: String,
    extra: String,
}
impl ListItem {
    pub fn clone(&self) -> ListItem {
        ListItem { name: self.name.to_string(), extra: self.extra.to_string() }
    }
}
impl Ord for ListItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}
impl PartialOrd for ListItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for ListItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for ListItem {}

pub(crate) enum NavDir {
    Up,
    Down,
    PageUp,
    PageDown,
}

#[allow(dead_code)]
pub(crate) struct VaultUx {
    // messages not handled by the main loop are routed here
    msg: Option<MessageEnvelope>,

    /// the content area
    content: Gid,
    gam: gam::Gam,

    /// screensize of the content area
    screensize: Point,
    margin: Point, // margin to edge of canvas

    /// our security token for making changes to our record on the GAM
    token: [u32; 4],

    /// current operation mode
    mode: VaultMode,

    /// list of all items to be displayed
    item_list: Vec::<ListItem>,
    /// list of items displayable after filtering
    filtered_list: Vec::<ListItem>,
    /// the index into the item_list that is selected
    selection_index: usize,

    /// pddb handle
    pddb: RefCell::<Pddb>,

    /// current font style
    style: GlyphStyle,
    item_height: i16,
    items_per_screen: i16,
}

pub(crate) const DEFAULT_FONT: GlyphStyle = GlyphStyle::Regular;
pub(crate) const FONT_LIST: [&'static str; 6] = [
    "regular", "mono", "cjk",
    "bold", "large", "small"
];
pub(crate) fn name_to_style(name: &str) -> Option<GlyphStyle> {
    match name {
        "regular" => Some(GlyphStyle::Regular),
        "mono" => Some(GlyphStyle::Monospace),
        "cjk" => Some(GlyphStyle::Cjk),
        "bold" => Some(GlyphStyle::Bold),
        "large" => Some(GlyphStyle::Large),
        "small" => Some(GlyphStyle::Small),
        _ => None
    }
}
fn style_to_name(style: &GlyphStyle) -> String {
    match style {
        GlyphStyle::Regular => "regular".to_string(),
        GlyphStyle::Monospace => "mono".to_string(),
        GlyphStyle::Cjk => "cjk".to_string(),
        GlyphStyle::Bold => "bold".to_string(),
        GlyphStyle::Large => "large".to_string(),
        GlyphStyle::Small => "small".to_string(),
        _ => "regular".to_string(),
    }
}

const TITLE_HEIGHT: i16 = 26;
const VAULT_CONFIG_DICT: &'static str = "vault.config";
const VAULT_CONFIG_KEY_FONT: &'static str = "fontstyle";
impl VaultUx {
    pub(crate) fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        let gam = gam::Gam::new(xns).expect("can't connect to GAM");

        let app_name_ref = gam::APP_NAME_VAULT;
        let token = gam.register_ux(UxRegistration {
            app_name: xous_ipc::String::<128>::from_str(app_name_ref),
            ux_type: gam::UxType::Chat,
            predictor: Some(xous_ipc::String::<64>::from_str(icontray::SERVER_NAME_ICONTRAY)),
            listener: sid.to_array(), // note disclosure of our SID to the GAM -- the secret is now shared with the GAM!
            redraw_id: VaultOp::Redraw.to_u32().unwrap(),
            gotinput_id: Some(VaultOp::Line.to_u32().unwrap()),
            audioframe_id: None,
            rawkeys_id: None,
            focuschange_id: Some(VaultOp::ChangeFocus.to_u32().unwrap()),
        }).expect("couldn't register Ux context for repl").unwrap();

        let content = gam.request_content_canvas(token).expect("couldn't get content canvas");
        let screensize = gam.get_canvas_bounds(content).expect("couldn't get dimensions of content canvas");
        gam.toggle_menu_mode(token).expect("couldnt't toggle menu mode");
        let margin = Point::new(4, 4);

        let pddb = pddb::Pddb::new();
        // TODO: put some informative message asking to mount the PDDB if it's not mounted, right now you just get a blank screen.
        pddb.is_mounted_blocking();
        // extract the style name from the settings, or update them if never initialized
        let mut style_name_bytes = [0u8; 32];
        let style_name = match pddb.get(
            VAULT_CONFIG_DICT,
            VAULT_CONFIG_KEY_FONT,
            None, true, true,
            Some(32), None::<fn()>
        ) {
            Ok(mut style_key) => {
                style_key.read(&mut style_name_bytes).ok();
                String::from_utf8_lossy(&style_name_bytes)
            }
            _ => panic!("PDDB access erorr"),
        };
        let style = match name_to_style(style_name.as_ref()) {
            Some(s) => s,
            None => {
                match pddb.get(
                    VAULT_CONFIG_DICT,
                    VAULT_CONFIG_KEY_FONT,
                    None, true, true,
                    Some(32), None::<fn()>
                ) {
                    Ok(mut style_key) => {
                        style_key.write(style_to_name(&DEFAULT_FONT).as_bytes()).ok();
                    }
                    _ => panic!("PDDB access erorr"),
                };
                GlyphStyle::Regular
            },
        };
        let available_height = screensize.y - TITLE_HEIGHT;
        let glyph_height = gam.glyph_height_hint(style).unwrap();
        let item_height = (glyph_height * 2) as i16 + margin.y * 2 + 2; // +2 because of the border width
        let items_per_screen = available_height / item_height;

        VaultUx {
            msg: None,
            content,
            gam,
            screensize,
            margin,
            token,
            mode: VaultMode::Fido,
            item_list: Vec::new(),
            selection_index: 0,
            filtered_list: Vec::new(),
            pddb: RefCell::new(pddb),
            style,
            item_height,
            items_per_screen,
        }
    }
    pub(crate) fn set_mode(&mut self, mode: VaultMode) {
        self.item_list.clear();
        match mode {
            VaultMode::Fido | VaultMode::Password => self.gen_fake_data(0),
            VaultMode::Totp => self.gen_fake_data(1),
        }
        self.item_list.sort();
        self.selection_index = 0;
        self.filter("");
        self.mode = mode;
    }
    pub(crate) fn set_glyph_style(&mut self, style: GlyphStyle) {
        self.pddb.borrow().delete_key(VAULT_CONFIG_DICT, VAULT_CONFIG_KEY_FONT, None)
        .expect("couldn't delete previous setting");

        match self.pddb.borrow().get(
            VAULT_CONFIG_DICT,
            VAULT_CONFIG_KEY_FONT,
            None, true, true,
            Some(32), None::<fn()>
        ) {
            Ok(mut style_key) => {
                style_key.write(style_to_name(&style).as_bytes()).ok();
            }
            _ => panic!("PDDB access erorr"),
        };
        self.style = style;
        let available_height = self.screensize.y - TITLE_HEIGHT;
        let glyph_height = self.gam.glyph_height_hint(self.style).unwrap();
        self.item_height = (glyph_height * 2) as i16 + self.margin.y * 2 + 2; // +2 because of the border width
        self.items_per_screen = available_height / self.item_height;
    }
    pub(crate) fn nav(&mut self, dir: NavDir) {
        match dir {
            NavDir::Up => {
                if self.selection_index > 0 {self.selection_index -= 1;}
            }
            NavDir::Down => {
                if self.selection_index < self.filtered_list.len() - 1 {
                    self.selection_index += 1;
                }
            }
            NavDir::PageUp => {
                if self.selection_index > self.items_per_screen as usize {
                    self.selection_index -= self.items_per_screen as usize;
                } else {
                    self.selection_index = 0;
                }
            }
            NavDir::PageDown => {
                if self.selection_index < self.filtered_list.len() - 1 - self.items_per_screen as usize {
                    self.selection_index += self.items_per_screen as usize;
                } else {
                    self.selection_index = self.filtered_list.len() - 1;
                }
            }
        }
    }
    /// accept a new input string
    pub(crate) fn input(&mut self, line: &str) -> Result<(), xous::Error> {
        log::info!("filtering by {}", line);
        self.filter(line);
        log::info!("filtered item count: {}", self.filtered_list.len());
        Ok(())
    }

    pub(crate) fn msg(&mut self, message: MessageEnvelope) {
        self.msg = Some(message);
    }

    fn clear_area(&self) {
        self.gam.draw_rectangle(self.content,
            Rectangle::new_with_style(Point::new(0, 0), self.screensize,
            DrawStyle {
                fill_color: Some(PixelColor::Light),
                stroke_color: None,
                stroke_width: 0
            }
        )).expect("can't clear content area");
    }
    // dummy function for now - but this is where the action happens when input events come
    pub (crate) fn update(&mut self, _was_callback: bool) {
        self.redraw().unwrap();
    }
    pub(crate) fn redraw(&mut self) -> Result<(), xous::Error> {
        self.clear_area();

        // ---- draw title area ----
        let mut title_text = TextView::new(self.content,
            graphics_server::TextBounds::CenteredTop(
                Rectangle::new(
                    Point::new(self.margin.x, 0),
                    Point::new(self.screensize.x - self.margin.x, TITLE_HEIGHT)
                )
            )
        );
        title_text.draw_border = false;
        title_text.clear_area = true;
        title_text.style = GlyphStyle::Large;
        match self.mode {
            VaultMode::Fido => write!(title_text, "FIDO").ok(),
            VaultMode::Totp => write!(title_text, "⏳1234").ok(),
            VaultMode::Password => write!(title_text, "🔐****").ok(),
        };
        self.gam.post_textview(&mut title_text).expect("couldn't post title");

        // ---- draw list body area ----
        // line up the list to justify to the bottom of the screen, based on the actual font requested
        let items_height = self.items_per_screen * self.item_height;
        let mut insert_at = 1 + self.screensize.y - items_height; // +1 to get the border to overlap at the bottom

        let page = self.selection_index as i16 / self.items_per_screen;
        let selected = self.selection_index as i16 % self.items_per_screen;
        for (index, item) in self.filtered_list[
            ((page as usize) * self.items_per_screen as usize).min(self.filtered_list.len()) ..
            ((1 + page as usize) * self.items_per_screen as usize).min(self.filtered_list.len())
        ].iter().enumerate() {
            if insert_at - 1 > self.screensize.y - self.item_height { // -1 because of the overlapping border
                break;
            }
            let mut box_text = TextView::new(self.content,
                graphics_server::TextBounds::BoundingBox(
                    Rectangle::new(
                        Point::new(0, insert_at),
                        Point::new(self.screensize.x, insert_at + self.item_height)
                    )
                )
            );
            box_text.draw_border = true;
            box_text.rounded_border = None;
            box_text.clear_area = true;
            box_text.style = self.style;
            box_text.margin = self.margin;
            if index == selected as usize {
                box_text.border_width = 4;
            }
            write!(box_text, "{}\n{}", item.name, item.extra).ok();
            self.gam.post_textview(&mut box_text).expect("couldn't post list item");

            insert_at += self.item_height;
        }

        log::trace!("vault app redraw##");
        self.gam.redraw().expect("couldn't redraw screen");
        Ok(())
    }

    pub(crate) fn raise_menu(&self) {
        self.gam.raise_menu(gam::APP_MENU_0_VAULT).expect("couldn't raise our submenu");
    }

    pub(crate) fn filter(&mut self, criteria: &str) {
        self.filtered_list.clear();
        for item in self.item_list.iter() {
            if item.name.starts_with(criteria) {
                self.filtered_list.push(item.clone());
            }
        }
        if self.selection_index >= self.filtered_list.len() {
            self.selection_index = self.filtered_list.len() - 1;
        }
    }

    // populates the display list with testing data
    pub(crate) fn gen_fake_data(&mut self, set: usize) {
        if set == 0 {
            self.item_list.push(ListItem { name: "test.com".to_string(), extra: "Used 5 mins ago".to_string() });
            self.item_list.push(ListItem { name: "google.com".to_string(), extra: "Never used".to_string() });
            self.item_list.push(ListItem { name: "my app".to_string(), extra: "Used 2 hours ago".to_string() });
            self.item_list.push(ListItem { name: "💎🙌".to_string(), extra: "Used 2 days ago".to_string() });
            self.item_list.push(ListItem { name: "百度".to_string(), extra: "Used 1 month ago".to_string() });
            self.item_list.push(ListItem { name: "duplicate.com".to_string(), extra: "Used 1 week ago".to_string() });
            self.item_list.push(ListItem { name: "duplicate.com".to_string(), extra: "Used 8 mins ago".to_string() });
            self.item_list.push(ListItem { name: "amazon.com".to_string(), extra: "Used 3 days ago".to_string() });
            self.item_list.push(ListItem { name: "ziggyziggyziggylongdomain.com".to_string(), extra: "Never used".to_string() });
            self.item_list.push(ListItem { name: "another long domain name.com".to_string(), extra: "Used 2 months ago".to_string() });
            self.item_list.push(ListItem { name: "bunniestudios.com".to_string(), extra: "Used 30 mins ago".to_string() });
            self.item_list.push(ListItem { name: "github.com".to_string(), extra: "Used 6 hours ago".to_string() });
        } else {
            self.item_list.push(ListItem { name: "gmail.com".to_string(), extra: "162 321".to_string() });
            self.item_list.push(ListItem { name: "google.com".to_string(), extra: "445 768".to_string() });
            self.item_list.push(ListItem { name: "my 图片 app".to_string(), extra: "982 111".to_string() });
            self.item_list.push(ListItem { name: "🍕🍔🍟🌭".to_string(), extra: "056 182".to_string() });
            self.item_list.push(ListItem { name: "百度".to_string(), extra: "111 111".to_string() });
            self.item_list.push(ListItem { name: "duplicate.com".to_string(), extra: "462 124".to_string() });
            self.item_list.push(ListItem { name: "duplicate.com".to_string(), extra: "462 124".to_string() });
            self.item_list.push(ListItem { name: "amazon.com".to_string(), extra: "842 012".to_string() });
            self.item_list.push(ListItem { name: "ziggyziggyziggylongdomain.com".to_string(), extra: "462 212".to_string() });
            self.item_list.push(ListItem { name: "github.com".to_string(), extra: "Used 6 hours ago".to_string() });
        }
    }
}