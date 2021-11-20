#[cfg(feature = "colored")]
use crate::colors::ColoredLevelConfig;
use crate::{FormatCallback};

/// A generic formatter that easily provides good defaults while being configurable
#[derive(Clone, Debug)]
pub struct FormatterBuilder {
    #[cfg(feature = "colored")]
    color_config: Option<ColoredLevelConfig>,
    #[cfg(feature = "chrono")]
    chrono: bool,
    level: bool,
    target: bool,
}

impl Default for FormatterBuilder {
    fn default() -> Self {
        FormatterBuilder {
            #[cfg(feature = "colored")]
            color_config: colored::control::SHOULD_COLORIZE.should_colorize().then(Default::default),
            #[cfg(feature = "chrono")]
            chrono: true,
            level: true,
            target: true,
        }
    }
}

impl FormatterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level(mut self, level: bool) -> Self {
        self.level = level;
        self
    }

    pub fn target(mut self, target: bool) -> Self {
        self.target = target;
        self
    }

    #[cfg(feature = "colored")]
    pub fn color(mut self, color: bool) -> Self {
        self.color_config = if color {
            self.color_config.or_else(Default::default)
        } else {
            None
        };
        self
    }

    #[cfg(feature = "colored")]
    pub fn color_config(
        mut self,
        modify_config: impl FnOnce(ColoredLevelConfig) -> ColoredLevelConfig,
    ) -> Self {
        self.color_config = self.color_config.map(modify_config);
        self
    }

    #[cfg(feature = "chrono")]
    pub fn chrono(mut self, chrono: bool) -> Self {
        self.chrono = chrono;
        self
    }

    #[rustfmt::skip]
    pub fn build(
        self,
    ) -> impl Fn(FormatCallback<'_>, &std::fmt::Arguments<'_>, &log::Record<'_>) + Sync + Send + 'static
    {
        move |out, message, record| {
            /* Type checking is hard */
            #[cfg(not(feature = "chrono"))]
            type TimeType = String;
            #[cfg(feature = "chrono")]
            type TimeType<'a> = chrono::format::DelayedFormat<chrono::format::StrftimeItems<'a>>;

            let time: Option<TimeType> = {
                #[cfg(feature = "chrono")]
                {
                    self.chrono
                        .then(|| chrono::Local::now().format("%Y-%m-%d %H:%M:%S,%3f"))
                }
                #[cfg(not(feature = "chrono"))]
                {
                    None
                }
            };

            /* Type checking is hard */
            #[cfg(not(feature = "colored"))]
            type LevelType = log::Level;
            #[cfg(feature = "colored")]
            type LevelType = crate::colors::WithFgColor<log::Level>;

            let level: Option<LevelType> = if self.level {
                #[cfg(feature = "colored")]
                {
                    Some(
                        self.color_config
                            .map(|config| config.color(record.level()))
                            .unwrap_or_else(|| crate::colors::WithFgColor {
                                text: record.level(),
                                color: None,
                            }),
                    )
                }
                #[cfg(not(feature = "colored"))]
                {
                    Some(record.level())
                }
            } else {
                None
            };

            let target = self.target.then(|| {
                if !record.target().is_empty() {
                    record.target()
                } else {
                    record.module_path().unwrap_or_default()
                }
            });

            /* Sadly we cannot store and compose std::fmt::Arguments due to lifetime issues.
             * In order to avoid unnecessary write calls nevertheless, we must enumerate all options
             */
            match (time, level, target) {
                (Some(time), Some(level), Some(target)) => out.finish(format_args!("{} {:<5} [{}] {}", time, level, target, message)),
                (Some(time), Some(level), None) => out.finish(format_args!("{} {:<5}: {}", time, level, message)),
                (Some(time), None, Some(target)) => out.finish(format_args!("{} [{}] {}", time, target, message)),
                (Some(time), None, None) => out.finish(format_args!("{} {}", time, message)),
                (None, Some(level), Some(target)) => out.finish(format_args!("{:<5} [{}] {}", level, target, message)),
                (None, None, Some(target)) => out.finish(format_args!("[{}] {}", target, message)),
                (None, Some(level), None) => out.finish(format_args!("{}: {}", level, message)),
                (None, None, None) => out.finish(format_args!("{}", message)),
            }
        }
    }
}
