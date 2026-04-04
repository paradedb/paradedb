require "active_record"
require "parade_db"

ParadeDB::Arel::Visitor.install!
ParadeDB::Arel::Predications.install!

module RailsSnippetHarness
  database_user = ENV.fetch("PARADEDB_USER", ENV.fetch("USER", "postgres"))
  database_password = ENV.fetch("PARADEDB_PASSWORD", "")
  database_host = ENV.fetch("PARADEDB_HOST", "localhost")
  database_port = ENV.fetch("PARADEDB_PORT", "28818")
  database_name = ENV.fetch("PARADEDB_DATABASE", "postgres")

  credentials = database_user.dup
  credentials << ":#{database_password}" unless database_password.empty?

  DATABASE_URL = "postgresql://#{credentials}@#{database_host}:#{database_port}/#{database_name}"

  class ApplicationRecord < ActiveRecord::Base
    self.abstract_class = true
  end

  class MockItem < ApplicationRecord
    include ParadeDB::Model

    self.table_name = "mock_items"

    has_many :orders,
      class_name: "RailsSnippetHarness::Order",
      foreign_key: :product_id,
      inverse_of: :mock_item
  end

  class Order < ApplicationRecord
    include ParadeDB::Model

    self.table_name = "orders"
    self.primary_key = "order_id"

    belongs_to :mock_item,
      class_name: "RailsSnippetHarness::MockItem",
      foreign_key: :product_id,
      inverse_of: :orders
  end

  module_function

  def establish_connection!
    ActiveRecord::Base.establish_connection(DATABASE_URL)
  end
end

RailsSnippetHarness.establish_connection!

MockItem = RailsSnippetHarness::MockItem
Order = RailsSnippetHarness::Order
