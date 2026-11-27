//! UI translations for the six supported interface languages.

use std::cell::Cell;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    En,
    #[default]
    ZhCn,
    Hi,
    Es,
    Fr,
    Ar,
}

impl Language {
    pub const ALL: [Language; 6] = [
        Language::En,
        Language::ZhCn,
        Language::Hi,
        Language::Es,
        Language::Fr,
        Language::Ar,
    ];

    pub fn native_name(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::ZhCn => "简体中文",
            Language::Hi => "हिन्दी",
            Language::Es => "Español",
            Language::Fr => "Français",
            Language::Ar => "العربية",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::ZhCn => "zh-cn",
            Language::Hi => "hi",
            Language::Es => "es",
            Language::Fr => "fr",
            Language::Ar => "ar",
        }
    }

    pub fn is_rtl(self) -> bool {
        matches!(self, Language::Ar)
    }
}

thread_local! {
    static CURRENT: Cell<Language> = const { Cell::new(Language::ZhCn) };
}

pub fn set_current(language: Language) {
    CURRENT.with(|current| current.set(language));
}

pub fn current() -> Language {
    CURRENT.with(Cell::get)
}

pub fn tr(source: &str) -> String {
    tr_lang(source, current())
}

pub fn tr_lang(source: &str, language: Language) -> String {
    if language == Language::ZhCn {
        return source.to_string();
    }
    for row in TRANSLATIONS {
        if row.zh == source {
            return match language {
                Language::En => row.en,
                Language::ZhCn => row.zh,
                Language::Hi => row.hi,
                Language::Es => row.es,
                Language::Fr => row.fr,
                Language::Ar => row.ar,
            }
            .to_string();
        }
    }
    if language != Language::ZhCn {
        for (zh, en) in EN_FALLBACK {
            if *zh == source {
                return (*en).to_string();
            }
        }
        for (zh, en) in DYNAMIC_PREFIXES {
            if let Some(rest) = source.strip_prefix(zh) {
                return format!("{en}{rest}");
            }
        }
        if let Some(rest) = source
            .strip_prefix("发现 ")
            .and_then(|value| value.strip_suffix(" 个问题，已定位到第一项。"))
        {
            return format!("Found {rest} issues; moved to the first one.");
        }
        if let Some(rest) = source
            .strip_prefix("已自动定位 ")
            .and_then(|value| value.strip_suffix(" 个构建输入。"))
        {
            return format!("Detected {rest} build inputs.");
        }
    }
    if let Some(rest) = source.strip_prefix("已定位工作区：") {
        return format!("{}{}", tr_lang("已定位工作区：", language), rest);
    }
    source.to_string()
}

struct Translation {
    zh: &'static str,
    en: &'static str,
    hi: &'static str,
    es: &'static str,
    fr: &'static str,
    ar: &'static str,
}

macro_rules! translations {
    ($(($zh:expr, $en:expr, $hi:expr, $es:expr, $fr:expr, $ar:expr)),+ $(,)?) => {
        &[$(Translation { zh: $zh, en: $en, hi: $hi, es: $es, fr: $fr, ar: $ar }),+]
    };
}

static DYNAMIC_PREFIXES: &[(&str, &str)] = &[
    ("配置已保存：", "Configuration saved: "),
    ("配置已导出：", "Configuration exported: "),
    ("已载入 AIP 配置：", "AIP configuration loaded: "),
    ("已导入自定义组件：", "Imported custom components: "),
    ("已应用市场组件：", "Applied marketplace component: "),
    ("安装器已生成：", "Installer created: "),
    (
        "GitHub 加速设置已保存：",
        "GitHub acceleration settings saved: ",
    ),
    ("开始下载 ", "Starting download: "),
    (
        "市场组件下载失败：",
        "Marketplace component download failed: ",
    ),
    ("保存配置失败：", "Failed to save configuration: "),
    ("构建准备失败：", "Build preparation failed: "),
];

static EN_FALLBACK: &[(&str, &str)] = &[
    ("载入 AIP 配置", "Load AIP configuration"),
    ("导出 AIP 配置", "Export AIP configuration"),
    ("选择构建目录", "Choose build directory"),
    ("选择 stub.exe", "Choose stub.exe"),
    ("选择 client.exe", "Choose client.exe"),
    ("选择 backend.exe", "Choose backend.exe"),
    ("选择 payload.zip", "Choose payload.zip"),
    ("选择客户端 assets 目录", "Choose client assets directory"),
    ("选择 packager.exe", "Choose packager.exe"),
    ("选择安装器输出位置", "Choose installer output"),
    ("选择应用图标", "Choose app icon"),
    ("选择背景图片", "Choose background image"),
    ("选择 PFX 签名证书", "Choose PFX signing certificate"),
    ("选择 signtool.exe", "Choose signtool.exe"),
    ("导入自定义运行组件目录", "Import custom runtime component directory"),
    ("自动定位", "Auto discovery"),
    ("config.toml 与固定命名的输入文件会写入这里；输出必须在目录外。", "config.toml and fixed-name inputs are written here; the output must be outside this directory."),
    ("目录不存在时会自动创建", "Created automatically if missing"),
    ("界面会把 stub、client、backend 同步为构建目录中的固定文件名。", "The GUI copies stub, client, and backend into fixed build-directory names."),
    ("必须是不带现有 Authenticode 证书表的未签名 stub", "Must be an unsigned stub without an existing Authenticode certificate table"),
    ("AIP 安装界面进程", "AIP installer UI process"),
    ("AIP 安装执行器", "AIP installation executor"),
    ("payload.zip 是安装载荷；客户端 assets 会递归合并到 build/assets。", "payload.zip is the installation payload; client assets are merged into build/assets."),
    ("ZIP 至少包含一个文件，只允许 store/deflate", "ZIP must contain at least one file and use only store/deflate"),
    ("客户端资源目录（assets，可选）", "Client assets directory (optional)"),
    ("PyInstaller 目录式构建的 assets 目录，内容会合并进构建资源", "PyInstaller onedir assets directory; its contents are merged into build assets"),
    ("同步是合并写入，不会在打包前删除构建目录中已有的资源文件。", "Synchronization is additive and does not delete existing build assets."),
    ("尚未载入组件目录。", "The component index has not been loaded."),
    ("正在通过 jsDelivr 获取组件目录...", "Loading the component index through jsDelivr..."),
    ("选择签名方式、配置 packager，并输出最终单文件安装器。", "Choose a signing method, configure packager, and produce the final single-file installer."),
    ("打包器负责最终 AIP 校验、签名和自解包验证。", "The packager performs final AIP validation, signing, and self-extraction verification."),
    ("默认查找同工作区的 target/release 或与 GUI 同目录", "Searches target/release in the workspace or next to the GUI"),
    ("必须是构建目录之外的 .exe 路径；父目录会自动创建", "Must be an .exe path outside the build directory; parent folders are created automatically"),
    ("同名文件存在时允许打包器原子替换。", "Allow the packager to atomically replace an existing file."),
    ("正式发布建议配置代码签名和 RFC 3161 时间戳。", "Code signing and an RFC 3161 timestamp are recommended for public releases."),
    ("证书存储", "Certificate store"),
    ("使用当前用户证书存储中的 SHA-1 指纹。", "Use a SHA-1 thumbprint from the current user certificate store."),
    ("PFX 文件", "PFX file"),
    ("从 PFX 文件签名，密码通过环境变量传入。", "Sign with a PFX file and pass its password through an environment variable."),
    ("适合本地调试；正式发布不推荐使用。", "Suitable for local debugging; not recommended for public releases."),
    ("未配置签名：正式发布不推荐使用。", "Signing is not configured; public release is not recommended."),
    ("当前用户证书存储中的 40 位十六进制指纹", "40 hexadecimal characters from the current user certificate store"),
    ("证书文件不能位于构建目录或安装目录内", "The certificate must not be inside the build or install directory"),
    ("只传环境变量名，GUI 与命令行日志不会显示密码值", "Only the environment variable name is passed; the password is never logged"),
    ("未填写 signtool 时，会从 PATH 与 Windows SDK 查找。", "If signtool is omitted, it is found through PATH or the Windows SDK."),
    ("未配置签名时不会使用此值", "Unused when signing is not configured"),
    ("留空则自动查找", "Leave empty to auto-detect"),
    ("留空时使用应用名称", "Uses the app name when empty"),
    ("输出安装器", "Output installer"),
    ("签名描述（可选）", "Signature description (optional)"),
    ("这些值会写入 config.toml 并用于窗口标题与快捷方式。", "These values are written to config.toml and used for the window title and shortcuts."),
    ("1–128 个字符，不能包含控制字符", "1–128 characters; control characters are not allowed"),
    ("推荐 .ico；也可由打包器转换 PNG、WebP、AVIF", "ICO recommended; the packager can convert PNG, WebP, or AVIF"),
    ("支持图片、特殊材质或纯色。背景只影响安装界面。", "Supports images, system materials, or solid colors. Background only affects the installer UI."),
    ("绝对路径，或相对于 config.toml 的路径", "Absolute path or a path relative to config.toml"),
    ("必须是 6 位 #RRGGBB；不接受 #RGB 或带 alpha 的颜色", "Must be a 6-digit #RRGGBB color; #RGB and alpha colors are not accepted"),
    ("未设置 background 键。安装器使用 AIP 规定的白色回退背景。", "background is not set; the AIP white fallback is used."),
    ("材质不受支持时，客户端按协议回退到白色背景，不中断安装。", "Unsupported materials fall back to white without interrupting installation."),
    ("推荐值不会随名称自动变化，避免覆盖手工修改。", "The recommended value does not follow name changes, preserving manual edits."),
    ("安装到当前用户的 FOLDERID_Programs。", "Installed under the current user's FOLDERID_Programs."),
    ("安装到当前用户的 FOLDERID_Desktop。", "Created under the current user's FOLDERID_Desktop."),
    ("两个选项都是安装器的初始值，用户可在安装时修改。", "Both options are initial installer values and can be changed during installation."),
    ("默认值不是任意脚本入口，安装行为仍由 backend 固定执行。", "Defaults are not arbitrary script entry points; backend still controls installation behavior."),
    ("安装位置会由 backend 展开环境变量、检查受保护目录与 reparse point。", "backend expands environment variables and checks protected directories and reparse points."),
    ("payload 中的程序、脚本与 DLL 不会被安装器执行。", "Executables, scripts, and DLLs in the payload are never executed by the installer."),
    ("客户端", "Client"),
    ("后端", "Backend"),
    ("官方", "Official"),
    ("社区", "Community"),
    ("构建任务已启动。", "Build task started."),
    ("发现 {} 个问题，已定位到第一项。", "Found {} issues; moved to the first one."),
    ("安装器已生成：{}", "Installer created: {}"),
    ("已载入 AIP 配置：{}", "AIP configuration loaded: {}"),
    ("配置已保存：{}", "Configuration saved: {}"),
    ("配置已导出：{}", "Configuration exported: {}"),
    ("已导入自定义组件：{}", "Imported custom components: {}"),
    ("已应用市场组件：{} {}", "Applied marketplace component: {} {}"),
    ("正在准备构建目录...", "Preparing the build directory..."),
];

static TRANSLATIONS: &[Translation] = translations![
    ("AIP 配置", "AIP configuration", "AIP कॉन्फ़िग", "Configuración AIP", "Configuration AIP", "إعدادات AIP"),
    ("Windows 可执行文件", "Windows executable", "Windows निष्पादन योग्य", "Ejecutable de Windows", "Exécutable Windows", "ملف Windows قابل للتنفيذ"),
    ("ZIP 载荷", "ZIP payload", "ZIP पेलोड", "Carga ZIP", "Charge ZIP", "حمولة ZIP"),
    ("图标", "Icon", "आइकन", "Icono", "Icône", "أيقونة"),
    ("证书存储", "Certificate store", "प्रमाणपत्र स्टोर", "Almacén de certificados", "Magasin de certificats", "مخزن الشهادات"),
    ("PFX 文件", "PFX file", "PFX फ़ाइल", "Archivo PFX", "Fichier PFX", "ملف PFX"),
    ("客户端", "Client", "क्लाइंट", "Cliente", "Client", "العميل"),
    ("后端", "Backend", "बैकएंड", "Backend", "Backend", "الخادم"),
    ("官方", "Official", "आधिकारिक", "Oficial", "Officiel", "رسمي"),
    ("社区", "Community", "समुदाय", "Comunidad", "Communauté", "مجتمع"),
    ("背景图片", "Background image", "पृष्ठभूमि छवि", "Imagen de fondo", "Image de fond", "صورة الخلفية"),
    ("必须是本地绝对盘符路径；支持 %VAR% 环境变量", "Must be an absolute local drive path; %VAR% environment variables are supported", "स्थानीय ड्राइव का पूर्ण पथ होना चाहिए; %VAR% समर्थित है", "Debe ser una ruta absoluta local; se admiten variables %VAR%", "Chemin absolu local requis ; variables %VAR% prises en charge", "يجب أن يكون مسار قرص محليًا مطلقًا؛ متغيرات %VAR% مدعومة"),
    ("从 jsDelivr 读取目录，从 GitHub Release 下载，并校验 SHA-256。", "Loads the index from jsDelivr, downloads from GitHub Releases, and verifies SHA-256.", "jsDelivr से सूचकांक, GitHub Releases से डाउनलोड और SHA-256 सत्यापन।", "Carga el índice desde jsDelivr, descarga desde GitHub Releases y verifica SHA-256.", "Charge l’index depuis jsDelivr, télécharge depuis GitHub Releases et vérifie SHA-256.", "يحمّل الفهرس من jsDelivr والتنزيلات من GitHub Releases ويتحقق من SHA-256."),
    ("例如：Example App", "Example: Example App", "उदाहरण: Example App", "Ejemplo: Example App", "Exemple : Example App", "مثال: Example App"),
    ("密码环境变量", "Password environment variable", "पासवर्ड एनवायरनमेंट वेरिएबल", "Variable de contraseña", "Variable du mot de passe", "متغير بيئة كلمة المرور"),
    ("默认安装位置", "Default install location", "डिफ़ॉल्ट इंस्टॉल स्थान", "Ubicación predeterminada", "Emplacement par défaut", "موقع التثبيت الافتراضي"),
    ("尚未设置输出", "Output not set", "आउटपुट सेट नहीं है", "Salida sin definir", "Sortie non définie", "المخرج غير محدد"),
    ("十六进制颜色", "Hex color", "हेक्स रंग", "Color hexadecimal", "Couleur hexadécimale", "لون سداسي عشري"),
    ("使用推荐路径", "Use recommended path", "अनुशंसित पथ उपयोग करें", "Usar ruta recomendada", "Utiliser le chemin recommandé", "استخدام المسار الموصى به"),
    ("市场目录加载失败", "Market index failed to load", "मार्केट सूचकांक लोड नहीं हुआ", "No se pudo cargar el índice", "Échec du chargement de l’index", "فشل تحميل فهرس السوق"),
    ("市场目录尚未加载", "Market index not loaded", "मार्केट सूचकांक लोड नहीं है", "Índice sin cargar", "Index non chargé", "فهرس السوق غير محمّل"),
    ("市场目录已加载", "Market index loaded", "मार्केट सूचकांक लोड हुआ", "Índice cargado", "Index chargé", "تم تحميل فهرس السوق"),
    ("市场目录正在加载", "Loading market index", "मार्केट सूचकांक लोड हो रहा है", "Cargando índice", "Chargement de l’index", "جارٍ تحميل فهرس السوق"),
    ("证书 SHA-1 指纹", "Certificate SHA-1 thumbprint", "प्रमाणपत्र SHA-1 थंबप्रिंट", "Huella SHA-1 del certificado", "Empreinte SHA-1 du certificat", "بصمة SHA-1 للشهادة"),
    ("组件作者可以通过 PR 提交 Release 条目。", "Component authors can submit Release entries by pull request.", "घटक लेखक PR से Release प्रविष्टि भेज सकते हैं।", "Los autores pueden enviar entradas de Release mediante PR.", "Les auteurs peuvent proposer une entrée Release via PR.", "يمكن للمؤلفين إرسال إدخالات Release عبر PR."),
    ("client 用它初始化输入框，backend 用它初始化会话状态。", "client uses it to initialize the input; backend uses it for session state.", "client इनपुट के लिए और backend सत्र स्थिति के लिए इसका उपयोग करते हैं।", "client inicializa el campo y backend el estado de sesión.", "client initialise le champ et backend l’état de session.", "يستخدم client هذا لتهيئة الحقل ويستخدمه backend لحالة الجلسة."),
    ("PFX 证书", "PFX certificate", "PFX प्रमाणपत्र", "Certificado PFX", "Certificat PFX", "شهادة PFX"),
    ("RFC 3161 时间戳 URL", "RFC 3161 timestamp URL", "RFC 3161 टाइमस्टैम्प URL", "URL de sello de tiempo RFC 3161", "URL d’horodatage RFC 3161", "عنوان طابع وقت RFC 3161"),
    ("signtool.exe（可选）", "signtool.exe (optional)", "signtool.exe (वैकल्पिक)", "signtool.exe (opcional)", "signtool.exe (facultatif)", "signtool.exe (اختياري)"),
    ("Windows 材质", "Windows material", "Windows सामग्री", "Material de Windows", "Matériau Windows", "مادة Windows"),
    ("界面语言已切换。", "Interface language changed.", "इंटरफ़ेस भाषा बदल गई।", "Idioma de la interfaz cambiado.", "Langue de l’interface modifiée.", "تم تغيير لغة الواجهة."),
    ("Anyinstaller 客户端与后端市场", "Anyinstaller Client & Backend Marketplace", "Anyinstaller क्लाइंट और बैकएंड मार्केट", "Mercado de clientes y backends de Anyinstaller", "Marché clients et backends Anyinstaller", "سوق عملاء وخوادم Anyinstaller"),
    ("通过 GitHub Release 分发的 AIP v1 客户端、后端与 stub。", "AIP v1 clients, backends, and stubs distributed through GitHub Releases.", "GitHub Releases के माध्यम से वितरित AIP v1 क्लाइंट, बैकएंड और stub।", "Clientes, backends y stubs AIP v1 distribuidos por GitHub Releases.", "Clients, backends et stubs AIP v1 distribués via GitHub Releases.", "عملاء وخوادم وstub بنظام AIP v1 تُوزّع عبر GitHub Releases."),
    ("官方 AIP v1 安装界面，使用 PySide6、Qt Quick 与 RinUI。", "Official AIP v1 installer UI built with PySide6, Qt Quick, and RinUI.", "PySide6, Qt Quick और RinUI से बना आधिकारिक AIP v1 इंस्टॉलर UI।", "Interfaz oficial AIP v1 con PySide6, Qt Quick y RinUI.", "Interface AIP v1 officielle avec PySide6, Qt Quick et RinUI.", "واجهة تثبيت AIP v1 رسمية باستخدام PySide6 وQt Quick وRinUI."),
    ("官方 AIP v1 安装执行器，负责安全解压、事务提交与快捷方式创建。", "Official AIP v1 installer executor for safe extraction, transactional commits, and shortcut creation.", "आधिकारिक AIP v1 इंस्टॉलर एक्ज़ीक्यूटर: सुरक्षित निष्कर्षण, लेन-देन और शॉर्टकट।", "Ejecutor oficial AIP v1 para extracción segura, confirmación transaccional y accesos directos.", "Exécuteur AIP v1 officiel pour extraction sécurisée, validation transactionnelle et raccourcis.", "منفّذ تثبيت AIP v1 رسمي للاستخراج الآمن والمعاملات والاختصارات."),
    ("Anyinstaller 官方单文件入口 stub。发布构建必须另行使用 Authenticode 签名流程。", "Official Anyinstaller single-file stub. Release builds still require Authenticode signing.", "आधिकारिक Anyinstaller एकल-फ़ाइल stub। रिलीज़ बिल्ड के लिए Authenticode हस्ताक्षर आवश्यक हैं।", "Stub oficial de archivo único de Anyinstaller. Las versiones públicas requieren firma Authenticode.", "Stub officiel Anyinstaller. Les versions publiques nécessitent une signature Authenticode.", "stub رسمي أحادي الملف من Anyinstaller. تتطلب إصدارات النشر توقيع Authenticode."),
    ("源码", "source", "स्रोत", "fuente", "source", "المصدر"),
    ("偏好设置", "Preferences", "प्राथमिकताएँ", "Preferencias", "Préférences", "التفضيلات"),
    ("AIP // 打包工作台", "AIP // Packager", "AIP // पैकेजर", "AIP // Empaquetador", "AIP // Empaqueteur", "AIP // الحازم"),
    ("自动检测当前出口 IP；国内开启 GitHub 加速，海外自动关闭；ipwho.is 不可用时默认按国内处理。", "Detect the current exit IP. Domestic routes enable GitHub acceleration, overseas routes disable it; ipwho.is failures use the domestic fallback.", "वर्तमान एग्ज़िट IP जाँचें। घरेलू मार्ग पर GitHub त्वरण चालू, विदेशी पर बंद; ipwho.is विफल होने पर घरेलू विकल्प।", "Detecta la IP de salida. Las rutas nacionales activan la aceleración y las internacionales la desactivan; si ipwho.is falla se usa el modo nacional.", "Détecte l’IP de sortie. Les routes nationales activent l’accélération GitHub, les routes internationales la désactivent ; en cas d’échec d’ipwho.is, le mode national est utilisé.", "يكشف IP الخروج. المسارات المحلية تفعّل تسريع GitHub والخارجية توقفه؛ عند فشل ipwho.is يُستخدم الوضع المحلي."),
    ("仅访问 ipwho.is，不发送账号、配置或本机文件。", "Only ipwho.is is contacted; no account, configuration, or local files are sent.", "केवल ipwho.is से संपर्क होता है; खाता, कॉन्फ़िग या स्थानीय फ़ाइल नहीं भेजी जाती।", "Solo se contacta con ipwho.is; no se envían cuentas, configuración ni archivos locales.", "Seul ipwho.is est contacté ; aucun compte, fichier ou configuration locale n’est envoyé.", "يُتصل بـ ipwho.is فقط؛ لا تُرسل حسابات أو إعدادات أو ملفات محلية."),
    ("例如 https://gh.meali.top/", "Example: https://gh.meali.top/", "उदाहरण: https://gh.meali.top/", "Ejemplo: https://gh.meali.top/", "Exemple : https://gh.meali.top/", "مثال: https://gh.meali.top/"),
    ("国内自动开启，海外自动关闭；仍可手动覆盖。下载失败时回退 GitHub 直连。", "Automatically on for domestic routes and off overseas; manual override remains available. Failed accelerated downloads fall back to GitHub.", "घरेलू में स्वतः चालू, विदेशी में बंद; मैन्युअल बदलाव संभव है। त्वरित डाउनलोड विफल होने पर GitHub पर लौटता है।", "Se activa en rutas nacionales y se desactiva en internacionales; puede cambiarse manualmente. Si falla la descarga acelerada se usa GitHub.", "Activation automatique en national, désactivation à l’international ; modification manuelle possible. En cas d’échec, retour à GitHub.", "يُفعّل تلقائيًا للمسارات المحلية ويُوقف للخارجية؛ يمكن التغيير يدويًا. عند فشل التسريع يعود إلى GitHub."),
    ("构建日志将在这里实时显示。", "Packaging logs will appear here in real time.", "पैकेजिंग लॉग यहाँ रीयलटाइम में दिखेंगे।", "Los registros de empaquetado aparecerán aquí en tiempo real.", "Les journaux d’empaquetage apparaîtront ici en temps réel.", "ستظهر سجلات الحزم هنا مباشرة."),
    ("编辑配置后即可一键整理输入并调用 packager。", "Edit the configuration, then prepare inputs and run packager in one step.", "कॉन्फ़िग बदलें, फिर एक चरण में इनपुट तैयार कर packager चलाएँ।", "Edita la configuración y ejecuta packager después de preparar las entradas.", "Modifiez la configuration, puis préparez les entrées et lancez packager.", "عدّل الإعدادات ثم جهّز المدخلات وشغّل packager."),
    ("本地未签名", "Unsigned locally", "स्थानीय रूप से अहस्ताक्षरित", "Sin firma local", "Non signé localement", "غير موقّع محليًا"),
    ("设置保存在本机，不影响 AIP config.toml。", "Settings are stored locally and do not modify AIP config.toml.", "सेटिंग्स स्थानीय रूप से सहेजी जाती हैं और AIP config.toml को नहीं बदलतीं।", "Los ajustes se guardan localmente y no modifican AIP config.toml.", "Les paramètres restent locaux et ne modifient pas AIP config.toml.", "تُحفظ الإعدادات محليًا ولا تغيّر AIP config.toml."),
    ("自动寻找 packager、stub、client、backend 与客户端 assets。", "Automatically find packager, stub, client, backend, and client assets.", "packager, stub, client, backend और क्लाइंट assets स्वतः खोजें।", "Busca automáticamente packager, stub, client, backend y recursos del cliente.", "Recherche automatiquement packager, stub, client, backend et ressources client.", "ابحث تلقائيًا عن packager وstub وclient وbackend وموارد العميل."),
    ("检测结果只填充输入路径，之后仍可逐个浏览并改成本机文件。", "Detection only fills input paths; you can still browse and replace them with local files.", "पहचान केवल पथ भरती है; बाद में स्थानीय फ़ाइलें चुन सकते हैं।", "La detección solo rellena rutas; después puede elegir archivos locales.", "La détection ne remplit que les chemins ; vous pouvez ensuite choisir des fichiers locaux.", "الكشف يملأ المسارات فقط، ويمكنك بعدها اختيار ملفات محلية."),
    ("设置和组件缓存位于用户数据目录，不写入 AIP 包。", "Settings and component caches live in the user data directory, not the AIP package.", "सेटिंग्स और घटक कैश उपयोगकर्ता डेटा फ़ोल्डर में हैं, AIP पैकेज में नहीं।", "Los ajustes y la caché viven en datos del usuario, no en el paquete AIP.", "Les paramètres et le cache résident dans les données utilisateur, pas dans le paquet AIP.", "تُحفظ الإعدادات وذاكرة المكونات في بيانات المستخدم لا في حزمة AIP."),
    ("已定位工作区：", "Workspace located: ", "वर्कस्पेस मिला: ", "Espacio de trabajo: ", "Espace de travail : ", "تم العثور على مساحة العمل: "),
    ("问题", "issues", "समस्याएँ", "problemas", "problèmes", "مشاكل"),
    ("出口 IP 尚未检测", "Exit IP not checked yet", "एग्ज़िट IP अभी जाँचा नहीं गया", "IP de salida sin comprobar", "IP de sortie non vérifiée", "لم يتم فحص IP الخروج بعد"),
    ("正在检测出口 IP...", "Checking exit IP...", "एग्ज़िट IP जाँचा जा रहा है...", "Comprobando IP de salida...", "Vérification de l’IP de sortie...", "جارٍ فحص IP الخروج..."),
    ("无法访问 ipwho.is，按国内处理", "ipwho.is unavailable; using domestic fallback", "ipwho.is उपलब्ध नहीं; घरेलू विकल्प", "ipwho.is no disponible; usando opción nacional", "ipwho.is indisponible ; repli national", "تعذر الوصول إلى ipwho.is؛ استخدام الخيار المحلي"),
    ("国内出口", "Domestic exit", "घरेलू एग्ज़िट", "Salida nacional", "Sortie nationale", "خروج محلي"),
    ("海外出口", "Overseas exit", "विदेशी एग्ज़िट", "Salida internacional", "Sortie internationale", "خروج دولي"),
    ("尚未检测", "Not checked", "अभी नहीं जाँचा", "Sin comprobar", "Non vérifiée", "لم يُفحص"),
    ("检测中", "Checking", "जाँच हो रही है", "Comprobando", "Vérification", "جارٍ الفحص"),
    ("国内 · 自动开启", "Domestic · auto on", "घरेलू · स्वतः चालू", "Nacional · automático", "National · auto", "محلي · تلقائي"),
    ("海外 · 自动关闭", "Overseas · auto off", "विदेशी · स्वतः बंद", "Internacional · automático", "International · auto", "دولي · تلقائي"),
    ("ipwho.is 不可用 · 默认国内", "ipwho.is unavailable · domestic fallback", "ipwho.is उपलब्ध नहीं · घरेलू", "ipwho.is no disponible · nacional", "ipwho.is indisponible · national", "ipwho.is غير متاح · محلي"),
    ("正在检测", "Checking", "जाँच जारी", "Comprobando", "Vérification", "جارٍ الفحص"),
    ("等待自动检测", "Waiting for auto detection", "स्वतः पहचान की प्रतीक्षा", "Esperando detección", "En attente de détection", "بانتظار الكشف التلقائي"),
    ("匹配", "Match", "मेल", "Coincidencia", "Correspondance", "مطابقة"),
    ("应用信息", "Application", "ऐप जानकारी", "Aplicación", "Application", "معلومات التطبيق"),
    ("安装默认值", "Installation defaults", "इंस्टॉल डिफ़ॉल्ट", "Valores de instalación", "Valeurs d’installation", "إعدادات التثبيت"),
    ("运行组件", "Runtime components", "रनटाइम घटक", "Componentes de ejecución", "Composants d’exécution", "مكونات التشغيل"),
    ("签名与打包", "Sign & package", "हस्ताक्षर और पैकेजिंग", "Firma y empaquetado", "Signature et empaquetage", "التوقيع والحزم"),
    ("设置", "Settings", "सेटिंग्स", "Ajustes", "Paramètres", "الإعدادات"),
    ("身份与外观", "Identity & appearance", "पहचान और रूप", "Identidad y apariencia", "Identité et apparence", "الهوية والمظهر"),
    ("安装行为", "Install behavior", "इंस्टॉल व्यवहार", "Comportamiento de instalación", "Comportement d’installation", "سلوك التثبيت"),
    ("市场、载荷与资源", "Market, payload & assets", "मार्केट, पेलोड और संसाधन", "Mercado, carga y recursos", "Marché, charge et ressources", "السوق والحمولة والموارد"),
    ("证书与输出", "Signing & output", "प्रमाणपत्र और आउटपुट", "Certificados y salida", "Certificats et sortie", "الشهادات والمخرجات"),
    ("出口 IP 与自动检测", "Exit IP & auto detection", "एग्ज़िट IP और स्वतः पहचान", "IP de salida y detección automática", "IP de sortie et détection automatique", "IP الخروج والكشف التلقائي"),
    ("编辑 AIP v1 的应用名称、图标与安装界面背景。", "Edit the AIP v1 app name, icon, and installer background.", "AIP v1 ऐप का नाम, आइकन और इंस्टॉलर पृष्ठभूमि बदलें।", "Edita el nombre, icono y fondo del instalador AIP v1.", "Modifiez le nom, l’icône et le fond de l’installateur AIP v1.", "عدّل اسم التطبيق وأيقونته وخلفية المثبّت في AIP v1."),
    ("设置安装器首次打开时的默认安装行为。", "Configure the installer's default behavior on first launch.", "इंस्टॉलर के पहले खुलने पर डिफ़ॉल्ट व्यवहार सेट करें।", "Configura el comportamiento inicial del instalador.", "Configurez le comportement initial de l’installateur.", "اضبط السلوك الافتراضي عند أول تشغيل للمثبّت."),
    ("从市场下载或导入本机组件，并整理 payload 与客户端资源。", "Download or import runtime components, then prepare the payload and client assets.", "मार्केट से डाउनलोड करें या स्थानीय घटक आयात करें, फिर payload और क्लाइंट संसाधन तैयार करें।", "Descarga o importa componentes y prepara la carga útil y los recursos del cliente.", "Téléchargez ou importez les composants, puis préparez la charge utile et les ressources client.", "نزّل المكونات أو استوردها، ثم جهّز الحمولة وموارد العميل."),
    ("管理 GitHub 加速、出口 IP 自动检测与本机组件定位。", "Manage GitHub acceleration, exit IP detection, and local component discovery.", "GitHub त्वरण, एग्ज़िट IP पहचान और स्थानीय घटक खोज प्रबंधित करें।", "Gestiona la aceleración de GitHub, la IP de salida y la detección local.", "Gérez l’accélération GitHub, l’IP de sortie et la détection locale.", "أدر تسريع GitHub وكشف IP الخروج والمكونات المحلية."),
    ("配置就绪", "Ready", "तैयार", "Listo", "Prêt", "جاهز"),
    ("未保存", "Unsaved", "सहेजा नहीं गया", "Sin guardar", "Non enregistré", "غير محفوظ"),
    ("本机设置", "Local settings", "स्थानीय सेटिंग्स", "Ajustes locales", "Paramètres locaux", "إعدادات محلية"),
    ("准备输入", "Preparing", "तैयारी हो रही है", "Preparando", "Préparation", "جارٍ التحضير"),
    ("正在打包", "Packaging", "पैकेज बन रहा है", "Empaquetando", "Empaquetage", "جارٍ الحزم"),
    ("构建成功", "Build succeeded", "बिल्ड सफल", "Compilación correcta", "Build réussi", "نجح البناء"),
    ("构建失败", "Build failed", "बिल्ड विफल", "Error de compilación", "Échec du build", "فشل البناء"),
    ("上一步", "Back", "पिछला", "Atrás", "Précédent", "السابق"),
    ("下一步", "Next", "अगला", "Siguiente", "Suivant", "التالي"),
    ("工作流程", "Workflow", "वर्कफ़्लो", "Flujo de trabajo", "Flux de travail", "سير العمل"),
    ("Windows 10 / 11", "Windows 10 / 11", "Windows 10 / 11", "Windows 10 / 11", "Windows 10 / 11", "Windows 10 / 11"),
    ("单文件 · Windows", "Single-file · Windows", "एकल फ़ाइल · Windows", "Archivo único · Windows", "Fichier unique · Windows", "ملف واحد · Windows"),
    ("构建状态", "Build status", "बिल्ड स्थिति", "Estado de compilación", "État du build", "حالة البناء"),
    ("实时", "Live", "लाइव", "En vivo", "En direct", "مباشر"),
    ("打包日志", "Packaging log", "पैकेजिंग लॉग", "Registro de empaquetado", "Journal d’empaquetage", "سجل الحزم"),
    ("等待构建", "Waiting", "प्रतीक्षा", "En espera", "En attente", "في الانتظار"),
    ("空闲", "Idle", "निष्क्रिय", "Inactivo", "Inactif", "خامل"),
    ("成功", "Success", "सफल", "Correcto", "Réussi", "نجاح"),
    ("失败", "Failed", "विफल", "Error", "Échec", "فشل"),
    ("应用", "Application", "एप्लिकेशन", "Aplicación", "Application", "التطبيق"),
    ("安装", "Install", "इंस्टॉल", "Instalar", "Installer", "تثبيت"),
    ("校验", "Validation", "सत्यापन", "Validación", "Validation", "التحقق"),
    ("签名", "Signing", "हस्ताक्षर", "Firma", "Signature", "التوقيع"),
    ("输入", "Inputs", "इनपुट", "Entradas", "Entrées", "المدخلات"),
    ("行", "lines", "पंक्तियाँ", "líneas", "lignes", "أسطر"),
    ("载入配置", "Load config", "कॉन्फ़िग लोड करें", "Cargar configuración", "Charger la configuration", "تحميل الإعدادات"),
    ("保存配置", "Save config", "कॉन्फ़िग सहेजें", "Guardar configuración", "Enregistrer la configuration", "حفظ الإعدادات"),
    ("验证", "Validate", "सत्यापित करें", "Validar", "Valider", "تحقق"),
    ("开始打包", "Build package", "पैकेज बनाएँ", "Crear paquete", "Créer le paquet", "أنشئ الحزمة"),
    ("正在打包…", "Packaging…", "पैकेज बन रहा है…", "Empaquetando…", "Empaquetage…", "جارٍ الحزم…"),
    ("准备就绪", "Ready", "तैयार", "Listo", "Prêt", "جاهز"),
    ("应用身份", "App identity", "ऐप पहचान", "Identidad de la aplicación", "Identité de l’application", "هوية التطبيق"),
    ("安装界面背景", "Installer background", "इंस्टॉलर पृष्ठभूमि", "Fondo del instalador", "Fond de l’installateur", "خلفية المثبّت"),
    ("应用名称", "App name", "ऐप नाम", "Nombre de la aplicación", "Nom de l’application", "اسم التطبيق"),
    ("应用图标（可选）", "App icon (optional)", "ऐप आइकन (वैकल्पिक)", "Icono de la aplicación (opcional)", "Icône de l’application (facultatif)", "أيقونة التطبيق (اختياري)"),
    ("默认白色", "Default white", "डिफ़ॉल्ट सफ़ेद", "Blanco predeterminado", "Blanc par défaut", "أبيض افتراضي"),
    ("图片", "Image", "छवि", "Imagen", "Image", "صورة"),
    ("纯色", "Solid color", "ठोस रंग", "Color sólido", "Couleur unie", "لون ثابت"),
    ("系统材质", "System material", "सिस्टम सामग्री", "Material del sistema", "Matériau système", "مادة النظام"),
    ("安装位置", "Install location", "इंस्टॉल स्थान", "Ubicación de instalación", "Emplacement d’installation", "موقع التثبيت"),
    ("快捷方式", "Shortcuts", "शॉर्टकट", "Accesos directos", "Raccourcis", "الاختصارات"),
    ("创建开始菜单文件夹", "Create Start Menu folder", "स्टार्ट मेनू फ़ोल्डर बनाएँ", "Crear carpeta en Inicio", "Créer un dossier dans le menu Démarrer", "إنشاء مجلد في قائمة ابدأ"),
    ("创建桌面快捷方式", "Create desktop shortcut", "डेस्कटॉप शॉर्टकट बनाएँ", "Crear acceso directo en el escritorio", "Créer un raccourci sur le bureau", "إنشاء اختصار على سطح المكتب"),
    ("AIP 安全边界", "AIP security boundary", "AIP सुरक्षा सीमा", "Límite de seguridad AIP", "Frontière de sécurité AIP", "حد الأمان AIP"),
    ("组件市场", "Component marketplace", "घटक मार्केट", "Mercado de componentes", "Marché des composants", "سوق المكونات"),
    ("刷新市场", "Refresh market", "मार्केट रीफ़्रेश करें", "Actualizar mercado", "Actualiser le marché", "تحديث السوق"),
    ("刷新中…", "Refreshing…", "रीफ़्रेश हो रहा है…", "Actualizando…", "Actualisation…", "جارٍ التحديث…"),
    ("选择市场组件后会自动填入下方运行组件路径。", "Selecting a market component fills the runtime paths below.", "मार्केट घटक चुनने पर नीचे रनटाइम पथ भर दिए जाएँगे।", "Al seleccionar un componente se rellenan las rutas de ejecución.", "La sélection d’un composant remplit les chemins d’exécution ci-dessous.", "اختيار مكوّن من السوق يملأ مسارات التشغيل أدناه."),
    ("使用缓存", "Use cached", "कैश उपयोग करें", "Usar caché", "Utiliser le cache", "استخدام المخزون"),
    ("已安装", "Installed", "इंस्टॉल है", "Instalado", "Installé", "مثبّت"),
    ("下载并使用", "Download and use", "डाउनलोड और उपयोग करें", "Descargar y usar", "Télécharger et utiliser", "نزّل واستخدم"),
    ("下载中…", "Downloading…", "डाउनलोड हो रहा है…", "Descargando…", "Téléchargement…", "جارٍ التنزيل…"),
    ("项目主页", "Project page", "प्रोजेक्ट पेज", "Página del proyecto", "Page du projet", "صفحة المشروع"),
    ("贡献说明", "Contribution guide", "योगदान गाइड", "Guía de contribución", "Guide de contribution", "دليل المساهمة"),
    ("应用后仍可改成本机路径。", "You can still switch to a local path afterwards.", "लागू करने के बाद स्थानीय पथ चुन सकते हैं।", "Después puede usar una ruta local.", "Vous pouvez ensuite choisir un chemin local.", "يمكنك اختيار مسار محلي بعد التطبيق."),
    ("重新定位", "Detect again", "फिर खोजें", "Detectar de nuevo", "Détecter à nouveau", "إعادة الكشف"),
    ("导入自定义", "Import custom", "कस्टम आयात करें", "Importar personalizado", "Importer personnalisé", "استيراد مخصص"),
    ("网络设置", "Network settings", "नेटवर्क सेटिंग्स", "Ajustes de red", "Paramètres réseau", "إعدادات الشبكة"),
    ("构建目录", "Build directory", "बिल्ड फ़ोल्डर", "Directorio de compilación", "Dossier de build", "مجلد البناء"),
    ("浏览", "Browse", "ब्राउज़ करें", "Examinar", "Parcourir", "استعراض"),
    ("载荷与客户端资源", "Payload and client assets", "पेलोड और क्लाइंट संसाधन", "Carga útil y recursos del cliente", "Charge utile et ressources client", "الحمولة وموارد العميل"),
    ("签名方式", "Signing method", "हस्ताक्षर विधि", "Método de firma", "Méthode de signature", "طريقة التوقيع"),
    ("签名细节", "Signing details", "हस्ताक्षर विवरण", "Detalles de firma", "Détails de signature", "تفاصيل التوقيع"),
    ("打包器与输出", "Packager and output", "पैकेजर और आउटपुट", "Empaquetador y salida", "Empaqueteur et sortie", "الحازم والمخرجات"),
    ("覆盖已有输出", "Overwrite existing output", "मौजूदा आउटपुट बदलें", "Sobrescribir salida", "Remplacer la sortie", "استبدال المخرج"),
    ("出口 IP 与下载线路", "Exit IP and download route", "एग्ज़िट IP और डाउनलोड मार्ग", "IP de salida y ruta de descarga", "IP de sortie et route de téléchargement", "IP الخروج ومسار التنزيل"),
    ("当前出口 IP", "Current exit IP", "वर्तमान एग्ज़िट IP", "IP de salida actual", "IP de sortie actuelle", "IP الخروج الحالي"),
    ("重新检测出口 IP", "Detect exit IP again", "एग्ज़िट IP फिर जाँचें", "Volver a detectar IP", "Redétecter l’IP", "إعادة كشف IP الخروج"),
    ("下载加速前缀", "Download acceleration prefix", "डाउनलोड त्वरण उपसर्ग", "Prefijo de aceleración", "Préfixe d’accélération", "بادئة تسريع التنزيل"),
    ("使用 GitHub 下载加速", "Use GitHub download acceleration", "GitHub डाउनलोड त्वरण उपयोग करें", "Usar aceleración de GitHub", "Utiliser l’accélération GitHub", "استخدام تسريع GitHub"),
    ("保存网络设置", "Save network settings", "नेटवर्क सेटिंग्स सहेजें", "Guardar ajustes de red", "Enregistrer les paramètres réseau", "حفظ إعدادات الشبكة"),
    ("立即自动检测", "Run auto detection", "तुरंत स्वतः पहचानें", "Detección automática ahora", "Lancer la détection", "تشغيل الكشف التلقائي"),
    ("退出", "Exit", "बाहर निकलें", "Salir", "Quitter", "خروج"),
    ("返回运行组件", "Back to runtime components", "रनटाइम घटकों पर लौटें", "Volver a componentes", "Retour aux composants", "العودة إلى مكونات التشغيل"),
    ("工作区自动定位", "Workspace auto discovery", "वर्कस्पेस स्वतः खोज", "Detección automática del espacio", "Détection automatique de l’espace", "الكشف التلقائي عن مساحة العمل"),
    ("重新自动定位组件", "Detect components again", "घटक फिर खोजें", "Volver a detectar componentes", "Redétecter les composants", "إعادة كشف المكونات"),
    ("本机数据", "Local data", "स्थानीय डेटा", "Datos locales", "Données locales", "البيانات المحلية"),
    ("设置文件", "Settings file", "सेटिंग्स फ़ाइल", "Archivo de ajustes", "Fichier de paramètres", "ملف الإعدادات"),
    ("组件缓存", "Component cache", "घटक कैश", "Caché de componentes", "Cache des composants", "ذاكرة المكونات"),
    ("市场目录", "Market index", "मार्केट सूचकांक", "Índice del mercado", "Index du marché", "فهرس السوق"),
    ("界面语言", "Interface language", "इंटरफ़ेस भाषा", "Idioma de la interfaz", "Langue de l’interface", "لغة الواجهة"),
    ("语言切换会立即生效，并保存在本机设置中。", "Language changes apply immediately and are saved locally.", "भाषा तुरंत बदलती है और स्थानीय रूप से सहेजी जाती है।", "El idioma se aplica al instante y se guarda localmente.", "La langue s’applique immédiatement et est enregistrée localement.", "يُطبّق تغيير اللغة فورًا ويُحفظ محليًا."),
    ("国内", "Domestic", "घरेलू", "Nacional", "National", "محلي"),
    ("海外", "Overseas", "विदेशी", "Internacional", "International", "خارجي"),
    ("自动开启", "Auto on", "स्वतः चालू", "Activación automática", "Activation auto", "تشغيل تلقائي"),
    ("自动关闭", "Auto off", "स्वतः बंद", "Desactivación automática", "Désactivation auto", "إيقاف تلقائي"),
    ("已找到", "Found", "मिला", "Encontrado", "Trouvé", "تم العثور عليه"),
    ("未找到", "Not found", "नहीं मिला", "No encontrado", "Introuvable", "غير موجود"),
    ("将创建", "Will be created", "बनाया जाएगा", "Se creará", "Sera créé", "سيتم إنشاؤه"),
    ("待生成", "Not generated", "अभी नहीं बना", "Pendiente", "À générer", "لم يُنشأ"),
    ("待选择", "Not selected", "चयनित नहीं", "Sin seleccionar", "Non sélectionné", "غير محدد"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_languages_have_native_names() {
        for language in Language::ALL {
            assert!(!language.native_name().is_empty());
            assert!(!language.code().is_empty());
        }
    }

    #[test]
    fn translates_known_ui_text() {
        assert_eq!(tr_lang("应用信息", Language::En), "Application");
        assert_eq!(tr_lang("应用信息", Language::Es), "Aplicación");
        assert_eq!(tr_lang("应用信息", Language::Ar), "معلومات التطبيق");
    }
}
