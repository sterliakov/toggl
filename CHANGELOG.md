## 0.3.5 (unreleased)

* bugfix: refresh icon (activity indicator) and clear error on profile change

## 0.3.4

This is a hotfix release for failed 0.3.3.

## 0.3.3

* bugfix: do not assume that running entry started this week
* feat: set `WM_CLASS` to `"toggl"` for the application window ([#107](https://github.com/sterliakov/toggl/issues/107))
* feat: prevent opening multiple windows, focus the existing one instead

## 0.3.2

This release is a fix for a failed release of 0.3.1.

## 0.3.1

* bugfix: removed unused dependencies
* feat: switched to trusted publishing

## 0.3.0

* feature: support multiple profiles
* chore: migrate to `tokio` and `reqwest` for dependency deduplication

## 0.2.5

* bugfix: reset running entry description after submission

## 0.2.4

* bugfix: description editor no longer handles keyboard shortcuts when not focused.
* feat: implement optimistic update to make fewer API requests and stay under API
  rate limits.
* feat: added tag editor

## 0.2.3

* feat: replaced old icon with a new one (shoutout to my wife Sofia who created
  it!)
* feat: added links to Toggl ToS and Privacy Policy

## 0.2.2

* bugfix: increased visibility of any errors.
* feat: added editor history (<kbd>Ctrl+Z</kbd> to undo).

## 0.2.1

* feat: added "Log out" button
* feat: dark mode support

## 0.2.0

* bugfix: fixed crash when no projects exist
* feat: improved top menu bar behaviour
* feat: added daily display of total tracked time
* feat: reduced the font size
* feat: improved entry editing controls:
	- added date and time pickers
	- replaced some text buttons with icons
	- added support for <kbd>Ctrl+Del</kbd>, <kbd>Ctrl+Backspace</kbd>,
	  <kbd>Ctrl+E</kbd> and <kbd>Ctrl+W</kbd> keyboard shortcuts in the entry
	  description editor
	- added support for <kbd>Ctrl+Enter</kbd> shortcut to quickly save the
	  entry after editing
* feat: added project badge to running entry row
* feat: added Cargo and NPM as distribution channels
* feat: added self-update support
* feat: date&time formats and default workspace selection are now synchronized
  with Toggl profile
* feat: beginning of week can be customized (synchronized with Toggl data)

## 0.1.2

* bugfix: fixed local time handling after DST transition
* chore: package dependencies updated

## 0.1.1

This is a bugfix release to patch a problem with "edit entry" button opening
the edit screen for a wrong entry.

## 0.1.0

This is an initial release.
