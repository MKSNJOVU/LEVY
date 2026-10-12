// Renders the summary it was given. Miss cents are read from miss_amount.
// The bar length uses money_medium and goal_amount only to compare two amounts.
(function () {
  var store = window.LevyFixtures;
  var state = {
    month: store.currentMonth,
    view: "month",
    accountId: null,
    editingGoal: false,
  };

  var monthNames = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

  function formatParts(cents) {
    var negative = cents < 0;
    var abs = Math.abs(cents);
    var dollars = Math.trunc(abs / 100);
    var fraction = abs % 100;
    return {
      sign: negative ? "-$" : "$",
      dollars: String(dollars).replace(/\B(?=(\d{3})+(?!\d))/g, ","),
      cents: "." + String(fraction).padStart(2, "0"),
    };
  }

  function formatDollars(cents) {
    var parts = formatParts(cents);
    return parts.sign + parts.dollars + parts.cents;
  }

  function monthLabel(ym) {
    var parts = ym.split("-");
    return monthNames[Number(parts[1]) - 1] + " " + parts[0].slice(2);
  }

  function dollarsToCents(text) {
    var cleaned = String(text).replace(/[$,\s]/g, "");
    var match = cleaned.match(/^(\d+)(?:\.(\d{0,2}))?$/);
    if (!match) {
      return null;
    }
    var dollars = Number(match[1]);
    var fraction = (match[2] || "").padEnd(2, "0");
    return dollars * 100 + Number(fraction);
  }

  function buildingIcon() {
    var svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("fill", "none");
    svg.setAttribute("stroke", "currentColor");
    svg.setAttribute("stroke-width", "1.5");
    svg.setAttribute("aria-hidden", "true");
    var roof = document.createElementNS("http://www.w3.org/2000/svg", "path");
    roof.setAttribute("d", "M4 20V10l8-6 8 6v10H4z");
    var door = document.createElementNS("http://www.w3.org/2000/svg", "path");
    door.setAttribute("d", "M9 20v-6h6v6");
    svg.append(roof, door);
    return svg;
  }

  function cardIcon() {
    var svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("fill", "none");
    svg.setAttribute("stroke", "currentColor");
    svg.setAttribute("stroke-width", "1.5");
    svg.setAttribute("aria-hidden", "true");
    var rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    rect.setAttribute("x", "3");
    rect.setAttribute("y", "6");
    rect.setAttribute("width", "18");
    rect.setAttribute("height", "12");
    var stripe = document.createElementNS("http://www.w3.org/2000/svg", "path");
    stripe.setAttribute("d", "M3 10h18");
    svg.append(rect, stripe);
    return svg;
  }

  function clear(node) {
    while (node.firstChild) {
      node.removeChild(node.firstChild);
    }
  }

  function renderStrip() {
    var strip = document.getElementById("month-strip");
    clear(strip);
    store.monthList().forEach(function (item) {
      var button = document.createElement("button");
      button.type = "button";
      button.dataset.month = item.month;
      button.textContent = monthLabel(item.month);
      button.setAttribute("aria-pressed", item.month === state.month ? "true" : "false");
      strip.append(button);
    });
  }

  function renderFigure(summary) {
    var hit = summary.miss_amount === 0;
    var parts = formatParts(summary.miss_amount);
    var result = document.getElementById("result");
    var figure = document.getElementById("figure");
    result.dataset.state = hit ? "hit" : "miss";
    figure.dataset.cents = String(summary.miss_amount);
    document.getElementById("status-word").textContent = hit ? "HIT" : "MISS";
    document.getElementById("sign").textContent = parts.sign;
    document.getElementById("dollars").textContent = parts.dollars;
    document.getElementById("cents").textContent = parts.cents;
  }

  function renderGoal(summary) {
    var read = document.getElementById("goal-read");
    var form = document.getElementById("goal-form");
    var button = document.getElementById("goal-button");
    button.textContent = formatDollars(summary.goal_amount);
    button.dataset.cents = String(summary.goal_amount);
    document.getElementById("goal-label").textContent = "Goal for " + monthLabel(state.month);
    read.hidden = state.editingGoal;
    form.hidden = !state.editingGoal;
  }

  function renderBar(summary) {
    var room = summary.money_medium - summary.goal_amount;
    var varying = summary.varying_expenses;
    var scale = Math.max(varying, room, 1);
    document.getElementById("bar-varying").style.width = (Math.max(0, varying) / scale) * 100 + "%";
    document.getElementById("bar-room").style.width = (Math.max(0, room) / scale) * 100 + "%";
    document.getElementById("varying-amt").textContent = formatDollars(varying);
    document.getElementById("varying-amt").dataset.cents = String(varying);
    document.getElementById("room-amt").textContent = formatDollars(room);
    document.getElementById("room-amt").dataset.cents = String(room);
  }

  function renderBuckets(rows) {
    var list = document.getElementById("buckets");
    clear(list);
    var max = 1;
    rows.forEach(function (row) {
      if (row.amount > max) {
        max = row.amount;
      }
    });
    rows.forEach(function (row) {
      var item = document.createElement("li");
      item.className = "bucket";
      var line = document.createElement("div");
      line.className = "bucket-row";
      var name = document.createElement("span");
      name.className = "bucket-name";
      name.textContent = row.category_name;
      var amount = document.createElement("span");
      amount.className = "bucket-amt";
      amount.dataset.cents = String(row.amount);
      amount.textContent = formatDollars(row.amount);
      var track = document.createElement("span");
      track.className = "bucket-track";
      var fill = document.createElement("span");
      fill.className = "bucket-fill";
      fill.style.width = (row.amount / max) * 100 + "%";
      track.append(fill);
      line.append(name, amount);
      item.append(line, track);
      list.append(item);
    });
  }

  function renderAccounts() {
    var list = document.getElementById("accounts");
    clear(list);
    store.accounts().forEach(function (account) {
      var button = document.createElement("button");
      button.type = "button";
      button.className = "account";
      button.dataset.accountId = account.id;
      var marks = document.createElement("span");
      marks.className = "marks";
      if (account.kind === "depository" || account.kind === "credit") {
        var kind = document.createElement("span");
        kind.className = "kind";
        kind.append(account.kind === "depository" ? buildingIcon() : cardIcon());
        marks.append(kind);
      }
      var initial = document.createElement("span");
      initial.className = "initial";
      initial.textContent = account.name.slice(0, 1).toUpperCase();
      marks.append(initial);
      var name = document.createElement("span");
      name.className = "account-name";
      name.textContent = account.name;
      var balance = document.createElement("span");
      balance.className = "account-balance";
      balance.textContent = formatDollars(account.balance);
      button.append(marks, name, balance);
      list.append(button);
    });
  }

  function renderTransactions() {
    var account = store.accounts().filter(function (item) {
      return item.id === state.accountId;
    })[0];
    document.getElementById("tx-title").textContent = account ? account.name : "Account";
    var rows = store.accountTransactions(state.accountId, state.month);
    var list = document.getElementById("tx-list");
    clear(list);
    rows.forEach(function (row) {
      var item = document.createElement("li");
      item.className = "tx";
      var copy = document.createElement("span");
      var desc = document.createElement("span");
      desc.className = "tx-desc";
      desc.textContent = row.description;
      var meta = document.createElement("span");
      meta.className = "tx-meta";
      meta.textContent = row.date + " · " + row.category;
      copy.append(desc, meta);
      var amount = document.createElement("span");
      amount.className = "tx-amt";
      amount.dataset.cents = String(row.amount);
      amount.textContent = formatDollars(row.amount);
      item.append(copy, amount);
      list.append(item);
    });
    document.getElementById("tx-empty").hidden = rows.length > 0;
  }

  function render() {
    renderStrip();
    var monthScreen = document.getElementById("month-screen");
    var txScreen = document.getElementById("transactions-screen");
    if (state.view === "transactions") {
      monthScreen.hidden = true;
      txScreen.hidden = false;
      renderTransactions();
      return;
    }
    monthScreen.hidden = false;
    txScreen.hidden = true;
    var summary = store.monthSummary(state.month);
    var empty = document.getElementById("empty");
    var result = document.getElementById("result");
    if (!summary) {
      empty.hidden = false;
      result.hidden = true;
      clear(document.getElementById("buckets"));
      renderAccounts();
      return;
    }
    empty.hidden = true;
    result.hidden = false;
    renderFigure(summary);
    renderGoal(summary);
    renderBar(summary);
    renderBuckets(store.monthBuckets(state.month));
    renderAccounts();
  }

  document.getElementById("month-strip").addEventListener("click", function (event) {
    var button = event.target.closest("button[data-month]");
    if (!button) {
      return;
    }
    state.month = button.dataset.month;
    state.editingGoal = false;
    state.view = "month";
    render();
  });

  document.getElementById("accounts").addEventListener("click", function (event) {
    var button = event.target.closest("button[data-account-id]");
    if (!button) {
      return;
    }
    state.accountId = button.dataset.accountId;
    state.view = "transactions";
    render();
    document.getElementById("back").focus();
  });

  document.getElementById("back").addEventListener("click", function () {
    var accountId = state.accountId;
    state.view = "month";
    render();
    var button = document.querySelector('[data-account-id="' + accountId + '"]');
    if (button) {
      button.focus();
    }
  });

  document.getElementById("goal-button").addEventListener("click", function () {
    state.editingGoal = true;
    render();
    var input = document.getElementById("goal-input");
    var summary = store.monthSummary(state.month);
    input.value = summary ? formatDollars(summary.goal_amount) : "";
    document.getElementById("goal-error").hidden = true;
    input.focus();
  });

  document.getElementById("goal-cancel").addEventListener("click", function () {
    state.editingGoal = false;
    render();
    document.getElementById("goal-button").focus();
  });

  document.getElementById("goal-form").addEventListener("submit", function (event) {
    event.preventDefault();
    var cents = dollarsToCents(document.getElementById("goal-input").value);
    var error = document.getElementById("goal-error");
    if (cents === null) {
      error.hidden = false;
      error.textContent = "Enter a dollar amount.";
      return;
    }
    error.hidden = true;
    store.setMonthGoal(state.month, cents);
    state.editingGoal = false;
    render();
  });

  render();
})();
