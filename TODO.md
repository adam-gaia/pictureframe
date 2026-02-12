# TODO
- Photo viewer should add a mat around the image, fitting the display.
  - Not sure yet, but could have mat settings (shape and size?) by image, editable in the admin page. I think it would be simpler not to have this
- Photo viewer should scale the image displayed to better fit the page.
- image tagging (giving images optional keywords to filter/group by) requires database migration to add support
  - short term: tagging done manually in admin panel
  - long term: local machine learning tagging program. Maybe some local llm already does this
- setting toggle for the viewer metadata overlay
- admin page should be able to edit the image metadata (title, notes, date taken, etc)
- admin page should show available space left on device!!
  - add some sort of guard where we can't upload images if we don't have some space threshold
- upload should take the files to upload then non-blocking process the files in the background. I think

## Hosting
- raspberry pi kiosk client
- server on the nas probably
