.pragma library
// A notification's actions, click and icon, from what its sender gave.
// Desktop Notifications: actions are identifier/label pairs, and the one
// keyed "default" is the action a click on the notification invokes, which
// need not be shown. Pure functions; Popups.qml draws the card.

// Every action but the default, in the sender's order: the card's buttons.
function buttons(actions) {
    return (actions || []).filter(a => a.identifier !== "default");
}

// The action a click on the card body invokes, or null: the click then
// dismisses the card.
function defaultAction(actions) {
    return (actions || []).find(a => a.identifier === "default") || null;
}

// The card's icon: the image hint, else the sender's icon name, else its
// desktop entry's icon, else none.
function iconSource(card) {
    return card.image || card.appIcon || card.desktopIcon || "";
}
