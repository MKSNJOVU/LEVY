// Stored closer results. The page displays these amounts and does not
// recompute a miss. setMonthGoal returns a scripted miss of 48000 cents.
(function () {
  var currentMonth = "2026-10";

  var summaries = {
    "2026-10": {
      income: 400000,
      fixed_expenses: 100000,
      varying_expenses: 40000,
      goal_amount: 280000,
      money_medium: 300000,
      miss_amount: 0,
    },
    "2026-09": {
      income: 400000,
      fixed_expenses: 100000,
      varying_expenses: 80000,
      goal_amount: 280000,
      money_medium: 300000,
      miss_amount: 60000,
    },
  };

  var buckets = {
    "2026-10": [{ category_name: "Food", amount: 40000 }],
    "2026-09": [{ category_name: "Food", amount: 80000 }],
  };

  var accounts = [
    {
      id: "checking",
      name: "Everyday checking",
      balance: 241050,
      kind: "depository",
    },
    {
      id: "card",
      name: "Travel card",
      balance: -84230,
      kind: "credit",
    },
  ];

  var transactions = [
    {
      account_id: "checking",
      month: "2026-10",
      date: "2026-10-01",
      description: "Payroll",
      amount: -210000,
      category: "INCOME",
      pending: false,
    },
    {
      account_id: "checking",
      month: "2026-10",
      date: "2026-10-02",
      description: "Pending transfer",
      amount: 1500,
      category: "TRANSFER_OUT",
      pending: true,
    },
    {
      account_id: "card",
      month: "2026-10",
      date: "2026-10-06",
      description: "Market",
      amount: 40000,
      category: "FOOD_AND_DRINK",
      pending: false,
    },
    {
      account_id: "card",
      month: "2026-10",
      date: "2026-10-07",
      description: "Held charge",
      amount: 999,
      category: "FOOD_AND_DRINK",
      pending: true,
    },
    {
      account_id: "card",
      month: "2026-09",
      date: "2026-09-12",
      description: "Market",
      amount: 80000,
      category: "FOOD_AND_DRINK",
      pending: false,
    },
  ];

  function shiftMonth(ym, delta) {
    var parts = ym.split("-");
    var index = Number(parts[0]) * 12 + (Number(parts[1]) - 1) + delta;
    var year = Math.floor(index / 12);
    var month = (index % 12) + 1;
    return year + "-" + String(month).padStart(2, "0");
  }

  function monthList() {
    var labels = [];
    var i;
    for (i = 0; i < 25; i += 1) {
      var month = shiftMonth(currentMonth, -i);
      labels.push({
        month: month,
        has_data: Object.prototype.hasOwnProperty.call(summaries, month),
      });
    }
    return labels;
  }

  function monthSummary(month) {
    if (!Object.prototype.hasOwnProperty.call(summaries, month)) {
      return null;
    }
    return summaries[month];
  }

  function monthBuckets(month) {
    return buckets[month] ? buckets[month].slice() : [];
  }

  function setMonthGoal(month, goalCents) {
    var current = summaries[month];
    if (!current) {
      return null;
    }
    var returned = {
      income: current.income,
      fixed_expenses: current.fixed_expenses,
      varying_expenses: current.varying_expenses,
      goal_amount: goalCents,
      money_medium: current.money_medium,
      miss_amount: 48000,
    };
    summaries[month] = returned;
    return returned;
  }

  function accountTransactions(accountId, month) {
    return transactions
      .filter(function (row) {
        return row.account_id === accountId && row.month === month && row.pending === false;
      })
      .map(function (row) {
        return {
          date: row.date,
          description: row.description,
          amount: row.amount,
          category: row.category,
        };
      });
  }

  window.LevyFixtures = {
    currentMonth: currentMonth,
    monthList: monthList,
    monthSummary: monthSummary,
    monthBuckets: monthBuckets,
    accounts: function () {
      return accounts.slice();
    },
    accountTransactions: accountTransactions,
    setMonthGoal: setMonthGoal,
  };
})();
