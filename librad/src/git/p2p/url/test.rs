// This file is part of radicle-link
// <https://github.com/radicle-dev/radicle-link>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;

#[test]
fn parse_url_with_socket() -> Result<(), ParseError> {
    let url = "rad-p2p://hyy95kazx9geddjpkw76zzwxmnw1ucy9gwaan8jg3y1aikwbyj1fy6@hybm3j53f9jz7ogxk3r6858wum9fys35bpjax9g7n4xwyj6uii1dbn.127.0.0.1:53371/hwd1yreyuskykcgk3dra6grwi34cedetn46ahy1cxcwnt319xkn9i4tx1wr.git";

    let _url: GitUrl = url.parse()?;

    Ok(())
}
