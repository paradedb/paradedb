using Microsoft.EntityFrameworkCore.Infrastructure;
using Microsoft.EntityFrameworkCore.Metadata;
using Microsoft.EntityFrameworkCore.Migrations;
using Microsoft.EntityFrameworkCore.Migrations.Operations;
using Microsoft.EntityFrameworkCore;
using NpgsqlTypes;
using ParadeDB.EntityFrameworkCore;
using ParadeDB.EntityFrameworkCore.Extensions;

var connectionString =
    $"Host={Environment.GetEnvironmentVariable("PARADEDB_HOST") ?? "localhost"};"
    + $"Port={Environment.GetEnvironmentVariable("PARADEDB_PORT") ?? "5432"};"
    + $"Database={Environment.GetEnvironmentVariable("PARADEDB_DATABASE") ?? "postgres"};"
    + $"Username={Environment.GetEnvironmentVariable("PARADEDB_USER") ?? "postgres"};"
    + $"Password={Environment.GetEnvironmentVariable("PARADEDB_PASSWORD") ?? "postgres"}";

var options = new DbContextOptionsBuilder<SnippetDbContext>()
    .UseNpgsql(connectionString, o => o.UseParadeDb())
    .Options;

await using var dbContext = new SnippetDbContext(options);
_ = dbContext.Model;

var designModel = dbContext.GetService<IDesignTimeModel>().Model;
var operations = dbContext.GetService<IMigrationsModelDiffer>()
    .GetDifferences(null, designModel.GetRelationalModel())
    .OfType<CreateIndexOperation>()
    .Where(operation => operation.FindAnnotation("ParadeDB:IndexFields") != null)
    .ToList();
if (Environment.GetEnvironmentVariable("DOCS_EXPECT_INDEX") == "true" && operations.Count == 0)
    throw new InvalidOperationException("No ParadeDB index DDL generated for the documented model");
foreach (var docsMigrationCommand in dbContext.GetService<IMigrationsSqlGenerator>().Generate(operations.Cast<MigrationOperation>().ToList(), designModel))
    await dbContext.Database.ExecuteSqlRawAsync(docsMigrationCommand.CommandText);
foreach (var operation in operations)
{
    var count = await dbContext.Database.SqlQuery<int>($"SELECT count(*)::int AS \"Value\" FROM pg_index WHERE indexrelid = to_regclass({operation.Name}) AND indisvalid AND indisready").SingleAsync();
    if (count != 1) throw new InvalidOperationException($"Missing or invalid index {operation.Name}");
}

// __PARADEDB_SNIPPET__

public sealed class SnippetDbContext(DbContextOptions<SnippetDbContext> options)
    : DbContext(options)
{
    public DbSet<MockItem> MockItems => Set<MockItem>();
    public DbSet<ArrayDemo> ArrayDemo => Set<ArrayDemo>();
    public DbSet<MockItemGeo> MockItemsGeo => Set<MockItemGeo>();
    public DbSet<Order> Orders => Set<Order>();

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder.Entity<MockItem>(entity =>
        {
            entity.ToTable("mock_items");
            entity.HasKey(item => item.Id);
            entity.Property(item => item.Id).HasColumnName("id");
            entity.Property(item => item.Description).HasColumnName("description");
            entity.Property(item => item.Rating).HasColumnName("rating");
            entity.Property(item => item.Category).HasColumnName("category");
            entity.Property(item => item.InStock).HasColumnName("in_stock");
            entity.Property(item => item.CreatedAt).HasColumnName("created_at");
            entity.Property(item => item.Metadata).HasColumnName("metadata").HasColumnType("jsonb");
            entity.Property(item => item.WeightRange).HasColumnName("weight_range");
            entity.Property(item => item.Embedding).HasColumnName("embedding").HasColumnType("vector(8)");
        });

        modelBuilder.Entity<ArrayDemo>(entity =>
        {
            entity.ToTable("array_demo");
            entity.HasKey(item => item.Id);
            entity.Property(item => item.Id).HasColumnName("id");
            entity.Property(item => item.Categories).HasColumnName("categories");
        });

        modelBuilder.Entity<Order>(entity =>
        {
            entity.ToTable("orders");
            entity.HasKey(order => order.OrderId);
            entity.Property(order => order.OrderId).HasColumnName("order_id");
            entity.Property(order => order.ProductId).HasColumnName("product_id");
            entity.Property(order => order.CustomerName).HasColumnName("customer_name");
        });

        modelBuilder.Entity<MockItemGeo>(entity =>
        {
            entity.ToTable("mock_items_geo");
            entity.HasKey(item => item.Id);
            entity.Property(item => item.Id).HasColumnName("id");
            entity.Property(item => item.Description).HasColumnName("description");
        });

        // __PARADEDB_MODEL_SNIPPET__
    }
}

public sealed class MockItem
{
    public int Id { get; set; }
    public string Description { get; set; } = "";
    public int? Rating { get; set; }
    public string Category { get; set; } = "";
    public bool InStock { get; set; }
    public DateTime CreatedAt { get; set; }
    public string Metadata { get; set; } = "{}";
    public NpgsqlRange<int> WeightRange { get; set; }
    public float[]? Embedding { get; set; }
}

public sealed class ArrayDemo
{
    public int Id { get; set; }
    public string[] Categories { get; set; } = [];
}

public sealed class Order
{
    public int OrderId { get; set; }
    public int ProductId { get; set; }
    public string CustomerName { get; set; } = "";
}

public sealed class MockItemGeo
{
    public long Id { get; set; }
    public string Description { get; set; } = "";
}
