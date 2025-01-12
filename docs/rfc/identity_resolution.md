# RFC: Identity Resolution

* Author: @kim
* Date: 2020-05-08
* Status: accepted
* Community discussion: https://radicle.community/t/rfc-identity-resolution/212

## Motivation

We have generalised the notion of "identity" in `radicle-link` to simply mean the
presence of a document at a conventional location within a git repository, where
the document is subject to certain verification rules. The hash of the initial
document is considered its stable identifier and encoded as a uniform resource
name (URN) of the form `rad:git:$HASH` (for git repositories). The URN is
supposed to be resolvable on the network into a top-level git repository of the
same name (`$HASH.git`), which is valid iff it contains said identity document,
and the document passes the verification rules.

This creates a problem: an identity document can be signed by one or more
top-level entities (users), whose identity documents may themselves be signed by
one or more top-level entities. Hence, in order to obtain the first repository
and verifying its identity, we need to recurse indefinitely trying to resolve,
clone, and verify the attesting entities' repositories.

This is rather impractical from an implementation point of view: the initial
clone has to block on a stack of asynchronous, high-latency network calls of
(potentially) arbitrary depth, and since there is no availability guarantee nor
-incentive, may still fail in the end, rendering the requested repository
unusable.

Hence, we are seeking to a. devise an incentive for the seeder to provide all
data necessary to resolve and verify a repository, and b. to reduce latency by
eliminating gossip queries and git fetches as much as possible.

## Overview

A lesser-known feature of the git suite are [namespaces], which are intended to
provide network access (push/pull) to a _subset_ of a single repository on the
server side. It turns out that this feature is not magical at all: a namespace
is simply a `refs` hierarchy under the `namespaces` refs category of the
"parent" repository. For example:

    refs/namespaces/foobar/refs/heads/master

The namespace is accessed by setting the `$GIT_NAMESPACE` environment variable,
or passing the `--namespace` parameter to the `git` executable. However, the
only commands which respect this are [git-upload-pack], [git-receive-pack], and
[git-http-backend]. When invoked namespaced, they will simply only consider the
`refs` hierarchy below the namespace (`foobar` in the example).

The fact that no other git command seems to be aware of namespaces ([git-gc]
would come to mind) is somewhat surprising, as it is otherwise not officially
endorsed that one may create other categories apart from `heads`, `tags`, and
`remotes` without any issues. Now that we know this, however, we can find
interesting ways to exploit it in interesting and barely kosher ways.

## Namespacing

Consider a `radicle-link` peer would store **all** git repositories it is
interested in in a single git repo, and made use of the namespaces feature to
partition it into logical, smaller repos, which can be checked out individually.

The namespacing scheme could look as follows:

    # Owner of this monorepo
    let PEER_ID;

    # Peer tracked by $PEER_ID, either directly or transitively
    let TRACKED_PEER_ID;

    # Identity hash of the project or user
    let IDENTITY;

    # Identity hashes of certifiers of $IDENTITY
    let CERTIFIER[1..];

    $PEER_ID/refs/
    `-- namespaces
        `-- $IDENTITY
            `-- refs
                |-- heads # <-- code branches owned by $PEER_ID go here
                |-- rad
                |   |-- id # <-- points to the identity document history
                |   |-- signed_refs # <-- signed refs of the peer
                |   |-- self # <-- points to the identity of $PEER_ID
                |   `-- ids
                |       |-- $CERTIFIER[1]
                |       `-- $CERTIFIER[2]
                `-- remotes
                    `-- $TRACKED_PEER_ID
                        |-- heads
                        `-- rad
                            |-- id
                            |-- signed_refs
                            |-- self
                            `-- ids
                                |-- $CERTIFIER[1]
                                `-- $CERTIFIER[2]

Note that the **owned** `$CERTIFIER[n]` refs (ie. not those of remotes) are
[symbolic refs], pointing to the `rad/id` branch of the respective namespace.
For example, if identity `A` is certified by identity `B`,
`refs/namespaces/A/refs/rad/ids/B` would contain:

    ref: refs/namespaces/B/refs/rad/id

Where tooling ensures that the certifier can only certify if the certifying
identity is present locally (and is logically valid for the certifier to use for
certifying). The symref ensures that the certifying identity can be updated in
one place (its logical repo), and stays up-to-date at all use sites without
maintenance.

The `rad/self` branch identifies `$PEER_ID`, ie. the `rad/id` branch of the
corresponding identity namespace. For example, if the identity of `$PEER_ID` is
`C`, `rad/self` within the context of `$IDENTITY` would be a symref:

    ref: refs/namespaces/C/rad/id

Any certifiers of the `self` identity must be included under `rad/ids/`. The
`rad/self` branch is equivalent to the contributor file in the [radicle-link
spec, rev1-draft], which is required iff the `refs/heads/` hierarchy of
`$PEER_ID` is non-empty (ie. it is permissible to omit it if `$PEER_ID` does not
publish any branches of their own).

## Fetching

Fetching (or cloning) would still happen on a per-`$IDENTITY` basis, as a
replication factor equal to the network size is not desirable. We also need to
map owned refs (`refs/heads`) to remotes, and should limit the number of refs
advertised by `git-upload-pack`.

In order to do so, `git-upload-pack --advertise-refs` transparently sets the
namespace to the requested repository identity. Due to the certifier symrefs,
the serving side advertises a "proof" (or perhaps better: "promise") to be able
to include all relevant data (the `rad/id` branches) in the packfile.

When negotiating the packfile, we do **not** namespace, such that the requester
can access the entire universe as seen by the server. The refspecs are computed
like this (`rad/refs` signature verification elided, which needs to come first,
incurring two additional network rountrips):

    # The set of all certifier identity hashes as found in the advertised refs,
    # i.e. `unique(refs/rad/ids/* || refs/remotes/**/rad/ids/*)`
    procedure certifiers -> Set CertifierIdentity;

    # The set of all transitively tracked peers
    let TRACKED_PEERS;

    # The currently connected-to peer
    let CONNECTED_PEER;

    for peer in $TRACKED_PEERS
        # We are connected to a tracked peers, so need to map owned to remote
        # branches
        if $peer == $CONNECTED_PEER
            # Code branches may be non-fast-forwarded
            +refs/namespaces/$IDENTITY/refs/heads/*:refs/namespaces/$IDENTITY/refs/remotes/$peer/refs/heads/*

            # Map the owned id and certifier branches
            refs/namespaces/$IDENTITY/rad/id*:refs/namespaces/$IDENTITY/refs/remotes/$peer/rad/id*

            # Also map the certifier identities from and to top-level repos.
            # Here, we're only interested in the branches owned by $peer.
            for certifier in certifiers()
                refs/namespaces/$certifier/rad/id*:refs/namespaces/$certifier/refs/remotes/$peer/rad/id*
            end
        else
            # Same as above, but $CONNECTED_PEER doesn't own the code branches
            # (but is -- possibly -- tracking $peer).
            +refs/namespaces/$IDENTITY/refs/remotes/$peer/heads/*:refs/namespaces/$IDENTITY/refs/remotes/$peer/refs/heads/*

            # Dito
            refs/namespaces/$IDENTITY/refs/remotes/$peer/rad/id*:refs/namespaces/$IDENTITY/refs/remotes/$peer/rad/id*

            # Map top-level identities (from and to remote $peer)
            for certifier in certifiers()
                refs/namespaces/$certifier/refs/remotes/$peer/rad/id*:refs/namespaces/$certifier/refs/remotes/$peer/rad/id*
            end
        end
    end

We can now, in a single packfile, receive a "mirror" of the logical remote
repository requested (modulo the mapping of remotely owned branches to
`refs/remotes`), _as well as_ all of the top-level logical repositories of all
certifiers required to verify the identity document(s). Additionally, also the
certifier identities can be verified, as we can resolve second-degree certifier
identities within the namespace of the respective certifier. This may still not
be sufficient, as recursion depth is not inherently limited by the identity
verification protocol -- it is, however, at the network protocol level, and it
is so at a reasonable depth which _should_ be sufficient for most purposes.

## Identity Resolution

As every top-level repository is strictly self-contained, the identity can be
verified without leaving the namespace.

Note that, although technically tolerated by the verification algorithm, we
reject history rewrites. This simplifies determining the latest known revision
of any given identity: across namespaces, multiple branches pointing to the same
identity exist, yet may point to different revisions. As their histories must be
linear, we can simply pick the most recent tip across namespaces.

## Working Copies

The astute reader will have noticed that our namespacing scheme takes the
liberty to introduce another refs category, `rad`, which is not well-known by
the git suite. The reason for this is that we can now trivially obtain a working
copy of just the logical repository we want to work with, while hiding "special"
branches:

    [remote "rad"]
        url = file://path/to/monorepo.git
        fetch = +refs/namespaces/$IDENTITY/refs/heads/*:refs/remotes/rad/refs/heads/*

One issue remains, however: as we're embracing the "bazaar" style of
development, we also want to see the branches of the peers we're tracking when
running `git branch` in the working copy -- however, a `PEER_ID` is not very
meaningful in this context. We need to inspect the `rad/self` identity metadata
in order to find nicknames, and generate human-readable remote tracking branch
names for the fetchspec.

Since the set of tracked peers may change over time, we cannot expect
the user to run a re-generate command periodically, modifying the git config of
the working copy. Luckily, [git-config] supports includes, so the remote
configuration can be managed entirely by `librad`, while in the working copy's
config it reduces to:

    [include]
        path = /path/to/managed.inc

Note that we also need to decide on the `HEAD` (ie. default branch to check
out), but since this is subject to workflow preferences, and dependent on the
verification result, a discussion is outside the scope of this document.

## Alternative Approaches

A similar effect, even exposing the same namespacing scheme on the git protocol
level, could be achieved by leaving top-level repositories standalone, but
mutually linking their object databases via [alternates]. An advantage would be
potentially more efficient [repack]s and resulting packfile layout.
Disadvantages include handling of symbolic refs, which would require
filesystem-level symbolic links, or a custom `refdb`, and handling of repository
deletion, which would require keeping track of a refcount, and prevent removal
before it has reached zero.

## Drawbacks

* The use of symrefs _below_ the `refs` hierarchy is somewhat unorthodox. As
  symrefs were invented to replace actual filesystem symbolic links (which are
  not entirely portable), it seems unlikely they would eventually stop working.
  If they did, we could still revert to symlinks again, and accept that this may
  limit platform choice for users.

* The `refs/rad` category is obviously also not entirely kosher, but since there
  are no hints in the git source code that `refs/namespaces` is treated
  specially, there is no reason to believe this would suddenly stop working. If
  it did, the only thing that would get more involved is the working copy branch
  mapping (which is managed).

* Lastly, with git being very much IO-bound, there are limits to (ab)using it as
  a giant monorepo. The precedence for this are Facebook moving to
  mercurial-based [eden], and Microsoft developing [VFSforGit]. However, there
  are also possibilities to mitigate scaling issues once they arise. One way is
  outlined in [Alternative Approaches](#alternative-approaches), but it is also
  feasible to replace the object and refs database backends entirely.

## Conclusion

Overall, the risks seem manageable, and the reduced complexity for obtaining,
updating, and verifying `radicle-link`-enabled git repositories appear to
outweigh them.

As Google and Facebook knew already, all source control problems can be solved
by a monorepo, and they can't be wrong, can they?


[namespaces]: https://git-scm.com/docs/gitnamespaces
[git-upload-pack]: https://git-scm.com/docs/git-upload-pack
[git-receive-pack]: https://git-scm.com/docs/git-receive-pack
[git-http-backend]: https://git-scm.com/docs/git-http-backend
[git-gc]: https://git-scm.com/docs/git-gc
[symbolic refs]: https://git-scm.com/docs/git-symbolic-ref
[git-config]: https://git-scm.com/docs/git-config
[eden]: https://github.com/facebookexperimental/eden
[VFSforGit]: https://github.com/microsoft/VFSforGit
[radicle-link spec, rev1-draft]: ../../spec/radicle-link.md
[alternates]: https://git-scm.com/docs/gitrepository-layout#Documentation/gitrepository-layout.txt-objectsinfoalternates
[repack]: https://git-scm.com/docs/git-repack
