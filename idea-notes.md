# Photo Handler — Product Idea

## Purpose

A local-first desktop application for reviewing, organizing, and importing personal photos and videos. It helps people move media from an input folder into a chosen library without repeatedly importing duplicates or losing track of files they chose to skip.

## Core user journey

1. On first launch, the user chooses the folder where their managed photo and video library will live.
2. The user selects a folder to import.
3. The app presents the folder's media for review, one item at a time (or in a similarly focused review flow).
4. For every item, the user can add tags and choose to import it or skip it.
5. Before importing, the app checks whether the item has already been handled and flags likely duplicates or visually similar media, along with useful context for making a decision.
6. The app records the user's decision. Imported files are added to the managed library with their tags; skipped files are remembered so they are not needlessly presented again.
7. When review is complete, the app offers to delete the original source files. The user then selects another folder to import.

## Library search journey

1. The user opens their managed media library.
2. They search or filter by one or more tags and available metadata, such as location, file size, media type, date, or other imported file details.
3. The app quickly shows the matching photos and videos, so the user can find media without browsing folders manually.

## Product principles

- **Local and private:** media and its catalogue stay on the user's computer.
- **User remains in control:** similarity detection recommends; the user decides what to keep, skip, or remove.
- **Safe by default:** original files are never deleted without an explicit confirmation after a completed import.
- **Memory of past decisions:** the app records both imported and skipped items to prevent repeated work.

## Duplicate and similarity handling

The app should distinguish between two cases:

- **Already handled:** the exact same file has been imported or previously skipped. It should not enter the normal review flow again.
- **Possibly related:** a different file may represent the same or a very similar photo. The app should show the match and relevant metadata, then let the user decide.

The internal approach can evolve, but it needs a durable local database that stores media identifiers, metadata, tags, and review decisions. This catalogue enables both duplicate handling and fast library search. Exact-file identification and visual similarity should be treated as separate capabilities.

## Scope to clarify later

- How photos and videos are organized in the destination library.
- Which tags and metadata are shown and searchable.
- How confident a similarity result must be before it is shown.
- The first practical approach to video similarity, which remains an open product and technical question.

## Initial success measure

A user can process a new media folder confidently: tag what they want to keep, avoid re-reviewing known files, identify likely duplicates, and finish knowing their original files remain under their control.
