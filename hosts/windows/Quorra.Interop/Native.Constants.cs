// quorra.h's numbers: every `#define QUORRA_…`, in the header's order and under its names. A number
// with a `u` suffix there is a `uint` here and one without is an `int`, which is the type of the
// argument or the result each is compared with.

namespace Quorra.Interop;

internal static partial class Native
{
    internal const uint QUORRA_ABI_VERSION = 2;

    internal const uint QUORRA_EVENT_KIND_COUNT = 30;

    internal const int QUORRA_OK = 0;
    internal const int QUORRA_NULL_ARGUMENT = 1;
    internal const int QUORRA_OUT_OF_RANGE = 2;
    internal const int QUORRA_WRONG_KIND = 3;
    internal const int QUORRA_BUFFER_TOO_SMALL = 4;
    internal const int QUORRA_NO_ANSWER = 5;
    internal const int QUORRA_NOT_UTF8 = 6;
    internal const int QUORRA_RENDER_REFUSED = 7;
    internal const int QUORRA_NUMBER_OUT_OF_RANGE = 8;
    internal const int QUORRA_UNREADABLE = 9;

    internal const uint QUORRA_EVENT_OPENED = 0;
    internal const uint QUORRA_EVENT_OPEN_FAILED = 1;
    internal const uint QUORRA_EVENT_PASSWORD_REQUIRED = 2;
    internal const uint QUORRA_EVENT_CLOSED = 3;
    internal const uint QUORRA_EVENT_PAGE_CHANGED = 4;
    internal const uint QUORRA_EVENT_NEEDS_RENDER = 5;
    internal const uint QUORRA_EVENT_DAMAGE = 6;
    internal const uint QUORRA_EVENT_OPEN_URI = 7;
    internal const uint QUORRA_EVENT_NEEDS_FILE = 8;
    internal const uint QUORRA_EVENT_TRANSITION = 9;
    internal const uint QUORRA_EVENT_DIRTY = 10;
    internal const uint QUORRA_EVENT_SAVED = 11;
    internal const uint QUORRA_EVENT_EXTRACTED = 12;
    internal const uint QUORRA_EVENT_REFUSED = 13;
    internal const uint QUORRA_EVENT_REPORTED = 14;

    internal const uint QUORRA_EVENT_SEARCHED = 15;

    internal const uint QUORRA_EVENT_ASKING = 16;
    internal const uint QUORRA_EVENT_WARNED = 17;
    internal const uint QUORRA_EVENT_ATTACHMENTS_CHANGED = 18;

    internal const uint QUORRA_EVENT_SUBMIT = 19;

    internal const uint QUORRA_EVENT_COPIED = 20;

    internal const uint QUORRA_EVENT_PRINTING = 21;

    internal const uint QUORRA_EVENT_ASKING_TO_RUN_SCRIPTS = 22;

    internal const uint QUORRA_EVENT_SCRIPT_ASKING = 23;

    internal const uint QUORRA_EVENT_SCRIPT_QUESTION_WITHDRAWN = 24;

    internal const uint QUORRA_EVENT_BEEP = 25;

    internal const uint QUORRA_EVENT_SIGNATURE_POLICIES_PUBLISHED = 26;

    internal const uint QUORRA_EVENT_CONSOLE = 27;

    internal const uint QUORRA_EVENT_MEDIA = 28;

    internal const uint QUORRA_EVENT_ARCHIVING = 29;

    internal const uint QUORRA_POINTER_MOVED = 0;
    internal const uint QUORRA_POINTER_PRESSED = 1;
    internal const uint QUORRA_POINTER_DRAGGED = 2;
    internal const uint QUORRA_POINTER_RELEASED = 3;

    internal const uint QUORRA_SELECT_ALL = 0;
    internal const uint QUORRA_SELECT_NONE = 1;

    internal const uint QUORRA_FOCUS_NEXT = 0;
    internal const uint QUORRA_FOCUS_PREVIOUS = 1;
    internal const uint QUORRA_FOCUS_NONE = 2;

    internal const uint QUORRA_RESTRICT_ON = 0;
    internal const uint QUORRA_RESTRICT_OFF = 1;

    internal const uint QUORRA_RESTRICT_ASK = 2;
    internal const uint QUORRA_RESTRICT_WARN = 3;

    internal const uint QUORRA_RESTRICT_INHERIT = 4;

    internal const uint QUORRA_RESTRICTED_COPY = 0;
    internal const uint QUORRA_RESTRICTED_ANNOTATE = 1;
    internal const uint QUORRA_RESTRICTED_FILL = 2;
    internal const uint QUORRA_RESTRICTED_PRINT = 3;
    internal const uint QUORRA_RESTRICTED_MODIFY = 4;
    internal const uint QUORRA_RESTRICTED_ASSEMBLE = 5;

    internal const uint QUORRA_RESTRICTED_PROCESS = 6;

    internal const uint QUORRA_RESTRICTED_PRINT_QUALITY = 7;

    internal const uint QUORRA_RESTRICTED_MEDIA = 8;

    internal const uint QUORRA_ACCEPT_REVOCATION_MUST_BE_GOOD = 0;
    internal const uint QUORRA_ACCEPT_UNKNOWN_REVOCATION = 1;

    internal const uint QUORRA_ATTACH_DOCUMENT = 0;
    internal const uint QUORRA_ATTACH_PAGE = 1;

    internal const uint QUORRA_LAYOUT_SINGLE_PAGE = 0;
    internal const uint QUORRA_LAYOUT_ONE_COLUMN = 1;
    internal const uint QUORRA_LAYOUT_TWO_COLUMN_LEFT = 2;
    internal const uint QUORRA_LAYOUT_TWO_COLUMN_RIGHT = 3;
    internal const uint QUORRA_LAYOUT_TWO_PAGE_LEFT = 4;
    internal const uint QUORRA_LAYOUT_TWO_PAGE_RIGHT = 5;

    internal const uint QUORRA_ORDER_LOGICAL = 0;
    internal const uint QUORRA_ORDER_PAGE_CONTENT = 1;

    internal const uint QUORRA_PRESENT_OFF = 0;
    internal const uint QUORRA_PRESENT_ON = 1;

    internal const uint QUORRA_DELEGATE_DRAWN = 0;
    internal const uint QUORRA_DELEGATE_DELEGATED = 1;

    internal const uint QUORRA_MARKUP_HIGHLIGHT = 0;
    internal const uint QUORRA_MARKUP_UNDERLINE = 1;
    internal const uint QUORRA_MARKUP_STRIKE_OUT = 2;
    internal const uint QUORRA_MARKUP_SQUIGGLY = 3;

    internal const uint QUORRA_PURPOSE_IMPORT_DATA = 0;
    internal const uint QUORRA_PURPOSE_TARGET_ROOT = 1;
    internal const uint QUORRA_PURPOSE_REMOTE_DOCUMENT = 2;
    internal const uint QUORRA_PURPOSE_NAMED_PAGE = 3;
    internal const uint QUORRA_PURPOSE_THREAD_DOCUMENT = 4;
    internal const uint QUORRA_PURPOSE_LAUNCH_DOCUMENT = 5;

    internal const uint QUORRA_OVERSIZE_REFUSED = 0;
    internal const uint QUORRA_OVERSIZE_LISTED = 1;

    internal const uint QUORRA_ANSWERED_SUBMISSION = 0;
    internal const uint QUORRA_ANSWERED_IMPORT = 1;

    internal const uint QUORRA_DATA_FDF = 0;
    internal const uint QUORRA_DATA_XFDF = 1;
    internal const uint QUORRA_DATA_OTHER = 2;

    internal const uint QUORRA_METHOD_GET = 0;
    internal const uint QUORRA_METHOD_POST = 1;

    internal const uint QUORRA_SCRIPTS_OFF = 0;
    internal const uint QUORRA_SCRIPTS_ASK = 1;
    internal const uint QUORRA_SCRIPTS_WARN = 2;
    internal const uint QUORRA_SCRIPTS_ON = 3;

    internal const uint QUORRA_SCRIPT_QUESTION_ALERT = 0;
    internal const uint QUORRA_SCRIPT_QUESTION_RESPONSE = 1;

    internal const uint QUORRA_SCRIPT_TEXT_MESSAGE = 0;
    internal const uint QUORRA_SCRIPT_TEXT_TITLE = 1;
    internal const uint QUORRA_SCRIPT_TEXT_DEFAULT = 2;
    internal const uint QUORRA_SCRIPT_TEXT_LABEL = 3;

    internal const uint QUORRA_ALERT_ICON_ERROR = 0;
    internal const uint QUORRA_ALERT_ICON_WARNING = 1;
    internal const uint QUORRA_ALERT_ICON_QUESTION = 2;
    internal const uint QUORRA_ALERT_ICON_STATUS = 3;
    internal const uint QUORRA_ALERT_BUTTONS_OK = 0;
    internal const uint QUORRA_ALERT_BUTTONS_OK_CANCEL = 1;
    internal const uint QUORRA_ALERT_BUTTONS_YES_NO = 2;
    internal const uint QUORRA_ALERT_BUTTONS_YES_NO_CANCEL = 3;

    internal const uint QUORRA_ALERT_BUTTON_OK = 1;
    internal const uint QUORRA_ALERT_BUTTON_CANCEL = 2;
    internal const uint QUORRA_ALERT_BUTTON_NO = 3;
    internal const uint QUORRA_ALERT_BUTTON_YES = 4;

    internal const uint QUORRA_SCRIPT_ANSWER_PRESSED = 0;
    internal const uint QUORRA_SCRIPT_ANSWER_TYPED = 1;
    internal const uint QUORRA_SCRIPT_ANSWER_UNANSWERABLE = 2;

    internal const uint QUORRA_KEY_A = 0;
    internal const uint QUORRA_KEY_C = 1;
    internal const uint QUORRA_KEY_F = 2;
    internal const uint QUORRA_KEY_H = 3;
    internal const uint QUORRA_KEY_K = 4;
    internal const uint QUORRA_KEY_L = 5;
    internal const uint QUORRA_KEY_M = 6;
    internal const uint QUORRA_KEY_O = 7;
    internal const uint QUORRA_KEY_P = 8;
    internal const uint QUORRA_KEY_R = 9;
    internal const uint QUORRA_KEY_S = 10;
    internal const uint QUORRA_KEY_T = 11;
    internal const uint QUORRA_KEY_W = 12;
    internal const uint QUORRA_KEY_Y = 13;
    internal const uint QUORRA_KEY_Z = 14;
    internal const uint QUORRA_KEY_ZERO = 15;
    internal const uint QUORRA_KEY_PLUS = 16;
    internal const uint QUORRA_KEY_MINUS = 17;
    internal const uint QUORRA_KEY_EQUALS = 18;
    internal const uint QUORRA_KEY_SLASH = 19;
    internal const uint QUORRA_KEY_QUESTION = 20;
    internal const uint QUORRA_KEY_ESCAPE = 21;
    internal const uint QUORRA_KEY_TAB = 22;
    internal const uint QUORRA_KEY_SPACE = 23;
    internal const uint QUORRA_KEY_ENTER = 24;
    internal const uint QUORRA_KEY_HOME = 25;
    internal const uint QUORRA_KEY_END = 26;
    internal const uint QUORRA_KEY_LEFT = 27;
    internal const uint QUORRA_KEY_RIGHT = 28;
    internal const uint QUORRA_KEY_UP = 29;
    internal const uint QUORRA_KEY_DOWN = 30;
    internal const uint QUORRA_KEY_PAGE_UP = 31;
    internal const uint QUORRA_KEY_PAGE_DOWN = 32;
    internal const uint QUORRA_KEY_COUNT = 33;

    internal const uint QUORRA_ACT_NOTHING = 0;
    internal const uint QUORRA_ACT_SAY = 1;
    internal const uint QUORRA_ACT_SCROLL_BY = 2;
    internal const uint QUORRA_ACT_COPY = 3;
    internal const uint QUORRA_ACT_FIND = 4;
    internal const uint QUORRA_ACT_PANEL = 5;
    internal const uint QUORRA_ACT_RESTRICTIONS = 6;
    internal const uint QUORRA_ACT_NOTICES = 7;
    internal const uint QUORRA_ACT_PRESENT = 8;
    internal const uint QUORRA_ACT_PRINT = 9;
    internal const uint QUORRA_ACT_ARCHIVE = 10;
    internal const uint QUORRA_ACT_CHOOSE_ARCHIVE = 11;
    internal const uint QUORRA_ACT_LEAVE_FULL_SCREEN = 12;
    internal const uint QUORRA_ACT_NEXT_LAYOUT = 13;
    internal const uint QUORRA_ACT_FIT_CONTROLS = 14;
    internal const uint QUORRA_ACT_FREE_TEXT = 15;
    internal const uint QUORRA_ACT_MEASURE = 16;
    internal const uint QUORRA_ACT_ABORT_DRAWING = 17;
    internal const uint QUORRA_ACT_NEXT_DOCUMENT = 18;
    internal const uint QUORRA_ACT_CLOSE_DOCUMENT = 19;
    internal const uint QUORRA_ACT_OPEN_DOCUMENT = 20;
    internal const uint QUORRA_ACT_SEPARATIONS = 21;
    internal const uint QUORRA_ACT_COUNT = 22;

    internal const uint QUORRA_CONTROL_ENTRY = 0;
    internal const uint QUORRA_CONTROL_CHECK = 1;
    internal const uint QUORRA_CONTROL_RADIO = 2;
    internal const uint QUORRA_CONTROL_PUSH = 3;
    internal const uint QUORRA_CONTROL_COMBO = 4;
    internal const uint QUORRA_CONTROL_LIST = 5;
    internal const uint QUORRA_CONTROL_SIGNATURE = 6;
    internal const uint QUORRA_CONTROL_UNSTATED = 7;
    internal const uint QUORRA_CONTROL_KIND_COUNT = 8;

    internal const uint QUORRA_ROW_ACTIVATE = 0;
    internal const uint QUORRA_ROW_TOGGLE = 1;
    internal const uint QUORRA_ROW_EXTRACT = 2;
    internal const uint QUORRA_ROW_INERT = 3;
    internal const uint QUORRA_ROW_KIND_COUNT = 4;

    internal const uint QUORRA_TEXT_QUALIFIED = 0;
    internal const uint QUORRA_TEXT_SHOWN = 1;
    internal const uint QUORRA_TEXT_PARTIAL = 2;
    internal const uint QUORRA_TEXT_LABEL = 3;
    internal const uint QUORRA_TEXT_EXPORT = 4;

    internal const uint QUORRA_PAGE_MODE_USE_NONE = 0;
    internal const uint QUORRA_PAGE_MODE_USE_OUTLINES = 1;
    internal const uint QUORRA_PAGE_MODE_USE_THUMBS = 2;
    internal const uint QUORRA_PAGE_MODE_FULL_SCREEN = 3;
    internal const uint QUORRA_PAGE_MODE_USE_OC = 4;
    internal const uint QUORRA_PAGE_MODE_USE_ATTACHMENTS = 5;
    internal const uint QUORRA_PAGE_MODE_COUNT = 6;

    internal const uint QUORRA_PREF_HIDE_TOOLBAR = 0;
    internal const uint QUORRA_PREF_HIDE_MENUBAR = 1;
    internal const uint QUORRA_PREF_HIDE_WINDOW_UI = 2;
    internal const uint QUORRA_PREF_FIT_WINDOW = 3;
    internal const uint QUORRA_PREF_CENTER_WINDOW = 4;
    internal const uint QUORRA_PREF_DISPLAY_DOC_TITLE = 5;
    internal const uint QUORRA_PREF_NON_FULL_SCREEN_PAGE_MODE = 6;
    internal const uint QUORRA_PREF_DIRECTION = 7;
    internal const uint QUORRA_PREF_VIEW_AREA = 8;
    internal const uint QUORRA_PREF_VIEW_CLIP = 9;
    internal const uint QUORRA_PREF_PRINT_AREA = 10;
    internal const uint QUORRA_PREF_PRINT_CLIP = 11;
    internal const uint QUORRA_PREF_PRINT_SCALING = 12;
    internal const uint QUORRA_PREF_DUPLEX = 13;
    internal const uint QUORRA_PREF_PICK_TRAY_BY_PDF_SIZE = 14;
    internal const uint QUORRA_PREF_NUM_COPIES = 15;
    internal const uint QUORRA_PREF_ENFORCE_PRINT_SCALING = 16;
    internal const uint QUORRA_PREF_PRINT_PAGE_RANGE = 17;
    internal const uint QUORRA_PREF_KEY_COUNT = 18;

    internal const uint QUORRA_DIRECTION_L2R = 0;
    internal const uint QUORRA_DIRECTION_R2L = 1;

    internal const uint QUORRA_BOUNDARY_MEDIA = 0;
    internal const uint QUORRA_BOUNDARY_CROP = 1;
    internal const uint QUORRA_BOUNDARY_BLEED = 2;
    internal const uint QUORRA_BOUNDARY_TRIM = 3;
    internal const uint QUORRA_BOUNDARY_ART = 4;

    internal const uint QUORRA_PRINT_SCALING_APP_DEFAULT = 0;
    internal const uint QUORRA_PRINT_SCALING_NONE = 1;
    internal const uint QUORRA_DUPLEX_SIMPLEX = 0;
    internal const uint QUORRA_DUPLEX_FLIP_SHORT_EDGE = 1;
    internal const uint QUORRA_DUPLEX_FLIP_LONG_EDGE = 2;

    internal const uint QUORRA_SHORTFALL_EMPTY_MAPPING = 0;
    internal const uint QUORRA_SHORTFALL_INCOMPLETE_TO_UNICODE = 1;
    internal const uint QUORRA_SHORTFALL_UNLISTED_NAME = 2;
    internal const uint QUORRA_SHORTFALL_UNNAMED_CID = 3;
    internal const uint QUORRA_SHORTFALL_UNADDRESSABLE_CID = 4;
    internal const uint QUORRA_SHORTFALL_UNNAMED_GLYPH = 5;
    internal const uint QUORRA_SHORTFALL_UNNAMED_TOTAL = 6;
    internal const uint QUORRA_SHORTFALL_WITHOUT_A_GLYPH = 7;
    internal const uint QUORRA_SHORTFALL_BLANK_GLYPH = 8;
    internal const uint QUORRA_SHORTFALL_UPRIGHT_VERTICAL_FORM = 9;

    internal const uint QUORRA_SHORTFALL_KIND_COUNT = 10;

    internal const uint QUORRA_THUMBNAIL_COLOUR_SPACE_UNPERMITTED = 1;
    internal const uint QUORRA_THUMBNAIL_SUBTYPE_UNPERMITTED = 2;

    internal const uint QUORRA_NOTE_TITLE = 0;
    internal const uint QUORRA_NOTE_CONTENTS = 1;
    internal const uint QUORRA_NOTE_MODIFIED = 2;

    internal const uint QUORRA_RICH_ALIGN_NONE = 0;
    internal const uint QUORRA_RICH_ALIGN_LEFT = 1;
    internal const uint QUORRA_RICH_ALIGN_CENTRE = 2;
    internal const uint QUORRA_RICH_ALIGN_RIGHT = 3;
    internal const uint QUORRA_RICH_ALIGN_JUSTIFY = 4;

    internal const uint QUORRA_RICH_TAB_LEFT = 0;
    internal const uint QUORRA_RICH_TAB_CENTRE = 1;
    internal const uint QUORRA_RICH_TAB_RIGHT = 2;
    internal const uint QUORRA_RICH_TAB_DECIMAL = 3;
    internal const uint QUORRA_RICH_TAB_AFTER = 4;
    internal const uint QUORRA_RICH_TAB_BEFORE = 5;

    internal const uint QUORRA_RICH_LEADER_NONE = 0;
    internal const uint QUORRA_RICH_LEADER_DOTS = 1;
    internal const uint QUORRA_RICH_LEADER_RULE = 2;
    internal const uint QUORRA_RICH_LEADER_CONTENT = 3;

    internal const uint QUORRA_RICH_RULE_SOLID = 0;
    internal const uint QUORRA_RICH_RULE_DASHED = 1;
    internal const uint QUORRA_RICH_RULE_DOTTED = 2;

    internal const uint QUORRA_ELEMENT_ROLE = 0;
    internal const uint QUORRA_ELEMENT_NAME = 1;
    internal const uint QUORRA_ELEMENT_LANGUAGE = 2;

    internal const uint QUORRA_BOX_STATED = 0;
    internal const uint QUORRA_BOX_DRAWN = 1;

    internal const uint QUORRA_SCOPE_ROW = 0;
    internal const uint QUORRA_SCOPE_COLUMN = 1;
    internal const uint QUORRA_SCOPE_BOTH = 2;

    internal const uint QUORRA_DECORATION_NONE = 0;
    internal const uint QUORRA_DECORATION_UNDERLINE = 1;
    internal const uint QUORRA_DECORATION_OVERLINE = 2;
    internal const uint QUORRA_DECORATION_LINE_THROUGH = 3;

    internal const uint QUORRA_POSITION_NORMAL = 0;
    internal const uint QUORRA_POSITION_SUPERSCRIPT = 1;
    internal const uint QUORRA_POSITION_SUBSCRIPT = 2;

    internal const uint QUORRA_COLLECTION_DETAILS = 0;
    internal const uint QUORRA_COLLECTION_TILE = 1;
    internal const uint QUORRA_COLLECTION_HIDDEN = 2;
    internal const uint QUORRA_COLLECTION_NAVIGATOR = 3;

    internal const uint QUORRA_INITIAL_CONTAINER = 0;
    internal const uint QUORRA_INITIAL_EMBEDDED = 1;
    internal const uint QUORRA_INITIAL_FIRST_FILE = 2;
    internal const uint QUORRA_INITIAL_EMPTY = 3;

    internal const uint QUORRA_COLLECTION_FIELD_TEXT = 0;
    internal const uint QUORRA_COLLECTION_FIELD_DATE = 1;
    internal const uint QUORRA_COLLECTION_FIELD_NUMBER = 2;
    internal const uint QUORRA_COLLECTION_FIELD_FILE_NAME = 3;
    internal const uint QUORRA_COLLECTION_FIELD_DESCRIPTION = 4;
    internal const uint QUORRA_COLLECTION_FIELD_MODIFICATION_DATE = 5;
    internal const uint QUORRA_COLLECTION_FIELD_CREATION_DATE = 6;
    internal const uint QUORRA_COLLECTION_FIELD_SIZE = 7;
    internal const uint QUORRA_COLLECTION_FIELD_COMPRESSED_SIZE = 8;
    internal const uint QUORRA_COLLECTION_FIELD_OTHER = 9;

    internal const uint QUORRA_COLUMN_NAME = 0;
    internal const uint QUORRA_COLUMN_KEY = 1;
    internal const uint QUORRA_COLUMN_SUBTYPE = 2;
    internal const uint QUORRA_COLUMN_VISIBLE = 1;
    internal const uint QUORRA_COLUMN_EDITABLE = 2;

    internal const uint QUORRA_NAVIGATOR_DETAILS = 0;
    internal const uint QUORRA_NAVIGATOR_TILE = 1;
    internal const uint QUORRA_NAVIGATOR_HIDDEN = 2;
    internal const uint QUORRA_NAVIGATOR_FILM_STRIP = 3;
    internal const uint QUORRA_NAVIGATOR_FREE_FORM = 4;
    internal const uint QUORRA_NAVIGATOR_LINEAR = 5;
    internal const uint QUORRA_NAVIGATOR_TREE = 6;
    internal const uint QUORRA_NAVIGATOR_CUSTOM = 7;

    internal const uint QUORRA_FOLDER_NAME = 0;
    internal const uint QUORRA_FOLDER_DESCRIPTION = 1;

    internal const uint QUORRA_FIELD_READ_ONLY = 1;
    internal const uint QUORRA_FIELD_REQUIRED = 2;
    internal const uint QUORRA_FIELD_NO_EXPORT = 4;
    internal const uint QUORRA_FIELD_MULTILINE = 8;
    internal const uint QUORRA_FIELD_PASSWORD = 16;
    internal const uint QUORRA_FIELD_FILE_SELECT = 32;
    internal const uint QUORRA_FIELD_DO_NOT_SPELL_CHECK = 64;
    internal const uint QUORRA_FIELD_DO_NOT_SCROLL = 128;
    internal const uint QUORRA_FIELD_COMB = 256;
    internal const uint QUORRA_FIELD_RICH_TEXT = 512;
    internal const uint QUORRA_FIELD_NO_TOGGLE_TO_OFF = 1024;
    internal const uint QUORRA_FIELD_RADIOS_IN_UNISON = 2048;
    internal const uint QUORRA_FIELD_ON = 4096;
    internal const uint QUORRA_FIELD_EDITABLE = 8192;
    internal const uint QUORRA_FIELD_MULTI_SELECT = 16384;
    internal const uint QUORRA_FIELD_COMMIT_ON_SEL = 32768;
    internal const uint QUORRA_FIELD_OBSCURED = 65536;

    internal const uint QUORRA_DIMENSION_HORIZONTAL = 0;
    internal const uint QUORRA_DIMENSION_VERTICAL = 1;
    internal const uint QUORRA_MOTION_INWARD = 0;
    internal const uint QUORRA_MOTION_OUTWARD = 1;

    internal const uint QUORRA_PAGE_INDEX = 0;
    internal const uint QUORRA_PAGE_FIRST = 1;
    internal const uint QUORRA_PAGE_LAST = 2;
    internal const uint QUORRA_PAGE_NEXT = 3;
    internal const uint QUORRA_PAGE_PREVIOUS = 4;
    internal const uint QUORRA_PAGE_RELATIVE = 5;

    internal const uint QUORRA_ZOOM_FIT_PAGE = 0;
    internal const uint QUORRA_ZOOM_FIT_WIDTH = 1;
    internal const uint QUORRA_ZOOM_FIT_HEIGHT = 2;
    internal const uint QUORRA_ZOOM_SCALE = 3;
    internal const uint QUORRA_ZOOM_IN = 4;
    internal const uint QUORRA_ZOOM_OUT = 5;

    internal const uint QUORRA_FORMAT_RGBA8 = 0;

    internal const uint QUORRA_MEASURE_SENTENCE = 0;
    internal const uint QUORRA_MEASURE_VIEWPORT = 1;
    internal const uint QUORRA_MEASURE_RATIO = 2;
    internal const uint QUORRA_MEASURE_LENGTH = 3;
    internal const uint QUORRA_MEASURE_AREA = 4;
    internal const uint QUORRA_MEASURE_ANGLE = 5;
    internal const uint QUORRA_MEASURE_SLOPE = 6;

    internal const uint QUORRA_OWED_SAID = 0;
    internal const uint QUORRA_OWED_REPAINT = 1;
    internal const uint QUORRA_OWED_TITLE = 2;
    internal const uint QUORRA_OWED_PASSWORD = 3;
    internal const uint QUORRA_OWED_QUESTION = 4;
    internal const uint QUORRA_OWED_CLIPBOARD = 5;
    internal const uint QUORRA_OWED_BEEP = 6;
    internal const uint QUORRA_OWED_PANELS = 7;
    internal const uint QUORRA_OWED_CHROME = 8;
    internal const uint QUORRA_OWED_PLACE = 9;
    internal const uint QUORRA_OWED_CONTROLS = 10;
    internal const uint QUORRA_OWED_POPUPS = 11;
    internal const uint QUORRA_OWED_STRIP = 12;
    internal const uint QUORRA_OWED_ARCHIVE = 13;
    internal const uint QUORRA_OWED_SCRIPT = 14;
    internal const uint QUORRA_OWED_SCRIPT_WITHDRAWN = 15;
    internal const uint QUORRA_OWED_PRINT = 16;
    internal const uint QUORRA_OWED_KIND_COUNT = 17;
    internal const uint QUORRA_REPAINT_FRAMES = 1;
    internal const uint QUORRA_REPAINT_PLACES = 2;
    internal const uint QUORRA_REPAINT_MARKS = 4;
    internal const uint QUORRA_POLICY_LINKS = 0;
    internal const uint QUORRA_POLICY_REMOTE_DOCUMENTS = 1;
    internal const uint QUORRA_POLICY_EMBEDDED_DOCUMENTS = 2;
    internal const uint QUORRA_POLICY_SUBMISSIONS = 3;
    internal const uint QUORRA_POLICY_SCRIPTS = 4;
    internal const uint QUORRA_POLICY_SEPARATIONS = 5;
    internal const uint QUORRA_POLICY_LAUNCHED_DOCUMENTS = 6;
    internal const uint QUORRA_LEVEL_REFUSE = 0;
    internal const uint QUORRA_LEVEL_ASK = 1;
    internal const uint QUORRA_LEVEL_WARN = 2;
    internal const uint QUORRA_LEVEL_PROCEED = 3;
    internal const uint QUORRA_READER_TRUST_ANCHORS = 0;
    internal const uint QUORRA_READER_REFERENCE_FILES = 1;
    internal const uint QUORRA_READER_NAME = 2;
    internal const uint QUORRA_READER_TITLE = 3;
    internal const uint QUORRA_READER_ORGANISATION = 4;
    internal const uint QUORRA_READER_LANGUAGE = 5;
    internal const uint QUORRA_WHEEL_LINES = 0;
    internal const uint QUORRA_WHEEL_PIXELS = 1;
    internal const uint QUORRA_TAB_CONTENTS = 0;
    internal const uint QUORRA_TAB_PAGES = 1;
    internal const uint QUORRA_TAB_LAYERS = 2;
    internal const uint QUORRA_TAB_FILES = 3;
    internal const uint QUORRA_TAB_ARTICLES = 4;
    internal const uint QUORRA_TAB_DOCUMENT = 5;
    internal const uint QUORRA_TAB_COUNT = 6;
}
