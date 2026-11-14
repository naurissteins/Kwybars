use super::{App, load};
use std::sync::Arc;

impl App {
    pub(in crate::app) fn follow_backgrounds(&mut self) {
        let Some(provider) = &mut self.sweetbg else {
            return;
        };
        let (selection, selected) = self.wayland.follow_selection();
        if provider.selection != selection {
            provider.selection = selection;
            provider.dirty = true;
            provider.rejected.clear();
        }
        let release: Vec<_> = provider
            .owned
            .iter()
            .filter(|name| {
                !selected.contains(name)
                    || !provider
                        .desired
                        .iter()
                        .any(|out| &out.name == *name && out.supported())
            })
            .cloned()
            .collect();
        for name in release {
            if let Some(socket) = &mut provider.socket
                && let Err(error) = socket.connection.release(&name)
            {
                provider.fail(error);
                break;
            }
            provider.owned.retain(|old| *old != name);
        }
        provider.load(selected);
        if provider.claiming.is_none() && provider.loading.is_none() && !provider.dirty {
            for (name, source) in self.wayland.ready_backgrounds() {
                if provider.owned.iter().any(|old| old == name)
                    || provider.rejected.iter().any(|old| old == name)
                {
                    continue;
                }
                if !provider
                    .images
                    .iter()
                    .any(|(old, image)| old == name && Arc::ptr_eq(image, source))
                {
                    continue;
                }
                if let Some(socket) = &mut provider.socket {
                    match socket.connection.claim(provider.revision, name) {
                        Ok(()) => provider.claiming = Some(name.to_owned()),
                        Err(error) => provider.fail(error),
                    }
                }
                break;
            }
        }
        if let Some(socket) = &mut provider.socket
            && let Err(error) = socket.connection.flush()
        {
            provider.fail(error);
        }
        self.wayland
            .set_followed_backgrounds(&provider.images, &provider.owned);
    }

    pub(super) fn sweetbg_loaded(&mut self, ready: load::Ready) {
        let Some(provider) = &mut self.sweetbg else {
            return;
        };
        if provider.epoch != ready.epoch {
            return;
        }
        provider.loading = None;
        if ready.request != provider.request || provider.dirty {
            self.follow_backgrounds();
            return;
        }
        for error in ready.errors {
            tracing::warn!("{error}; keeping the last good wallpaper");
        }
        let mut images = ready.images;
        for (name, source) in &provider.images {
            if provider.owned.contains(name) && !images.iter().any(|(next, _)| next == name) {
                images.push((name.clone(), Arc::clone(source)));
            }
        }
        provider.images = images;
        provider.known = ready.known;
        provider.watch_images();
        self.follow_backgrounds();
        self.render();
    }
}
