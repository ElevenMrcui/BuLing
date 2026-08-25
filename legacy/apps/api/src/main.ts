import "reflect-metadata";
import { NestFactory } from "@nestjs/core";
import { FastifyAdapter, NestFastifyApplication } from "@nestjs/platform-fastify";
import { ValidationPipe } from "@nestjs/common";
import cors from "@fastify/cors";
import { AppModule } from "./app.module.js";
import { logger } from "./common/logger.js";

async function bootstrap() {
  const app = await NestFactory.create<NestFastifyApplication>(
    AppModule,
    new FastifyAdapter({ logger: false }),
    { bufferLogs: true },
  );
  await app.register(cors as any, { origin: true, credentials: true });
  app.setGlobalPrefix("api");
  app.useGlobalPipes(
    new ValidationPipe({ whitelist: true, forbidNonWhitelisted: true, transform: true }),
  );
  const port = Number(process.env.API_PORT ?? 13500);
  await app.listen({ port, host: "0.0.0.0" });
  logger.info({ port }, "不令 API 已启动");
}

bootstrap().catch((err) => {
  logger.fatal({ err }, "API 启动失败");
  process.exit(1);
});
