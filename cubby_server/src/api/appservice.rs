use regex::RegexSet;
use ruma::api::appservice::Registration;

/// Compiled regular expressions for a namespace.
#[derive(Clone, Debug)]
pub(crate) struct NamespaceRegex {
    pub(crate) exclusive: Option<RegexSet>,
    pub(crate) non_exclusive: Option<RegexSet>,
}

#[derive(Clone, Debug)]
pub(crate) struct RegistrationInfo {
    pub(crate) registration: Registration,
    pub(crate) users: NamespaceRegex,
    pub(crate) aliases: NamespaceRegex,
    pub(crate) rooms: NamespaceRegex,
}
